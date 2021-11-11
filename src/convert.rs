use std::{
    collections::HashMap,
    io::{stdin, stdout, Write},
    iter::zip,
    path::Path,
};

use chrono::Duration;
use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    execution_engine::JitFunction,
    module::Module,
    passes::{PassManager, PassManagerBuilder},
    targets::{InitializationConfig, Target, TargetMachine},
    types::IntType,
    values::{FunctionValue, IntValue, PhiValue},
    AddressSpace, IntPredicate, OptimizationLevel,
};

type Label<'a> = (BasicBlock<'a>, BasicBlock<'a>);

pub struct Converter<'a> {
    context: &'a Context,
    module: Module<'a>,
    main: FunctionValue<'a>,
    builder: Builder<'a>,
    variables: Vec<IntValue<'a>>,
    phis: Vec<(Vec<PhiValue<'a>>, Label<'a>)>,
    mapping: HashMap<&'a str, usize>,
    one: IntValue<'a>,
    zero: IntValue<'a>,
    l64: IntType<'a>,
    block: BasicBlock<'a>,
}

impl<'a> Converter<'a> {
    pub fn new(varib: Vec<&'a str>, inputs: &Vec<&'a str>, context: &'a Context) -> Converter<'a> {
        let module: Module<'a> = context.create_module("bbvm");
        let l64 = context.i64_type();
        let one = l64.const_int(1, false);
        let zero = l64.const_zero();
        let main = module.add_function(
            "main",
            context
                .void_type()
                .fn_type(&vec![l64.into(); inputs.len()], false),
            None,
        );
        let block = context.append_basic_block(main, "entry");
        let builder = context.create_builder();
        builder.position_at_end(block);

        let mut variables = vec![l64.const_int(0, false); varib.len()];

        let phis = vec![];
        let mut mapping = HashMap::new();
        for v in varib.iter().enumerate() {
            mapping.insert(v.1.clone(), v.0);
        }
        for (input, param) in zip(inputs, main.get_params()) {
            variables[mapping[input]] = param.into_int_value();
        }
        Converter {
            context,
            module,
            main,
            builder,
            variables,
            phis,
            mapping,
            one,
            zero,
            l64,
            block,
        }
    }

    // var = var + 1
    pub fn add_incr<'b: 'a>(&mut self, var: &'b str) -> () {
        let pos = self.mapping[&var];

        self.variables[pos] = self
            .builder
            .build_int_add(self.variables[pos], self.one, "incr");
    }

    // if var != 0 {
    //   var = var - 1
    // }
    pub fn add_decr<'b: 'a>(&mut self, var: &'b str) -> () {
        let pos = self.mapping[&var];

        let current = self.variables[pos];

        let cmp = self
            .builder
            .build_int_compare(IntPredicate::EQ, current, self.zero, "cmp_to_0");

        let main = self.main;

        let skip = self.context.append_basic_block(main, "alreadyZero");
        let no_skip = self.context.append_basic_block(main, "notZero");
        self.builder.build_conditional_branch(cmp, skip, no_skip);

        self.builder.position_at_end(no_skip);
        let new_var = self.builder.build_int_nuw_sub(current, self.one, "decr");
        self.builder.build_unconditional_branch(skip);

        self.builder.position_at_end(skip);
        let res = self.builder.build_phi(self.l64, "result");
        res.add_incoming(&[(&current, self.block), (&new_var, no_skip)]);

        self.block = skip;

        self.variables[pos] = res.as_basic_value().into_int_value();
    }

    // var = 0
    pub fn add_clear<'b: 'a>(&mut self, var: &'b str) -> () {
        self.variables[self.mapping[&var]] = self.zero;
    }

    // to = from
    pub fn add_copy<'b: 'a>(&mut self, from: &'b str, to: &'b str) -> () {
        self.variables[self.mapping[&to]] = self.variables[self.mapping[&from]];
    }

    pub fn add_while<'b: 'a>(&mut self, var: &'b str, check: i128) -> () {
        let main = self.main;
        let lop = self.context.append_basic_block(main, "loop");
        self.builder.build_unconditional_branch(lop);
        self.builder.position_at_end(lop);

        let phis = self
            .variables
            .iter()
            .map(|var| {
                let rf = self.builder.build_phi(self.l64, "whilePhi");
                rf.add_incoming(&[(var, self.block)]);
                rf
            })
            .collect::<Vec<PhiValue>>();

        self.variables = phis
            .iter()
            .map(|phi| phi.as_basic_value().into_int_value())
            .collect::<Vec<IntValue>>();

        let cmp = self.builder.build_int_compare(
            IntPredicate::EQ,
            self.variables[self.mapping[&var]],
            self.l64.const_int(check as u64, false),
            "exitCondition",
        );
        let inner_loop = self.context.append_basic_block(main, "innerLoop");
        let exit = self.context.append_basic_block(main, "loopExit");
        self.builder.build_conditional_branch(cmp, exit, inner_loop);
        self.builder.position_at_end(inner_loop);

        self.block = inner_loop;
        self.phis.push((phis, (lop, exit)));
    }

    pub fn add_end(&mut self) -> () {
        let (phis, (start, end)) = self
            .phis
            .pop()
            .expect("ERROR: Phis list empty (too many \"end\"s?)");
        self.builder.build_unconditional_branch(start);
        self.builder.position_at_end(end);
        for (phi, var) in zip(&phis, &self.variables) {
            phi.add_incoming(&[(var, self.block)]);
        }
        self.variables = phis
            .iter()
            .map(|phi| phi.as_basic_value().into_int_value())
            .collect();
        self.block = end;
    }
    pub fn add_eof<'b>(&'b mut self) -> () {
        if self.phis.len() > 0 {
            panic!("Too many opening while loops!")
        }
        let fun = self.context.void_type().fn_type(
            &[
                self.context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .into(),
                self.l64.into(),
            ],
            false,
        );
        let printf = self.module.add_function("printf", fun, None);
        for var in &self.mapping {
            let fmt = self
                .builder
                .build_global_string_ptr(format!("{}: %lld\n", var.0).as_str(), "");
            self.builder.build_call(
                printf,
                &[fmt.as_pointer_value().into(), self.variables[*var.1].into()],
                "printf",
            );
        }

        self.builder.build_return(None);

        if let Err(e) = self.module.verify() {
            eprintln!("{}", e.to_str().unwrap());
            panic!("Module has errors");
        }
    }

    pub fn optimise(&mut self) -> bool {
        let pm_builder = PassManagerBuilder::create();
        pm_builder.set_optimization_level(OptimizationLevel::Aggressive);
        let pass_manager = PassManager::create(());
        pm_builder.populate_module_pass_manager(&pass_manager);
        pass_manager.run_on(&self.module)
    }

    pub fn run(&mut self, inputs: Vec<&'a str>) -> Duration {
        let execution_engine = self
            .module
            .create_jit_execution_engine(OptimizationLevel::Aggressive)
            .expect("Unable to create execution engine");
        unsafe {
            match inputs[..] {
                [] => {
                    println!("-----");

                    let start = chrono::Utc::now();
                    let main: JitFunction<'a, unsafe extern "C" fn() -> ()> = execution_engine
                        .get_function("main")
                        .expect("Unable to load function");
                    main.call();
                    println!("-----");
                    chrono::Utc::now() - start
                }
                [a] => {
                    println!("-----");
                    print!("{}: ", a);
                    stdout().flush().unwrap();
                    let mut a = String::new();
                    stdin().read_line(&mut a).unwrap();
                    let a = a.trim().parse().unwrap();
                    println!("-----");

                    let start = chrono::Utc::now();
                    let main: JitFunction<'a, unsafe extern "C" fn(u64) -> ()> = execution_engine
                        .get_function("main")
                        .expect("Unable to load function");
                    main.call(a);
                    println!("-----");
                    chrono::Utc::now() - start
                }
                [a, b] => {
                    println!("-----");
                    print!("{}: ", a);
                    stdout().flush().unwrap();
                    let mut a = String::new();
                    stdin().read_line(&mut a).unwrap();
                    let a = a.trim().parse().unwrap();
                    print!("{}: ", b);
                    stdout().flush().unwrap();
                    let mut b = String::new();
                    stdin().read_line(&mut b).unwrap();
                    let b = b.trim().parse().unwrap();
                    println!("-----");

                    let start = chrono::Utc::now();
                    let main: JitFunction<'a, unsafe extern "C" fn(u64, u64) -> ()> =
                        execution_engine
                            .get_function("main")
                            .expect("Unable to load function");
                    main.call(a, b);
                    println!("-----");
                    chrono::Utc::now() - start
                }
                [a, b, c] => {
                    println!("-----");
                    print!("{}: ", a);
                    stdout().flush().unwrap();
                    let mut a = String::new();
                    stdin().read_line(&mut a).unwrap();
                    let a = a.trim().parse().unwrap();
                    print!("{}: ", b);
                    stdout().flush().unwrap();
                    let mut b = String::new();
                    stdin().read_line(&mut b).unwrap();
                    let b = b.trim().parse().unwrap();
                    print!("{}: ", c);
                    stdout().flush().unwrap();
                    let mut c = String::new();
                    stdin().read_line(&mut c).unwrap();
                    let c = c.trim().parse().unwrap();
                    println!("-----");

                    let start = chrono::Utc::now();
                    let main: JitFunction<'a, unsafe extern "C" fn(u64, u64, u64) -> ()> =
                        execution_engine
                            .get_function("main")
                            .expect("Unable to load function");
                    main.call(a, b, c);
                    println!("-----");
                    chrono::Utc::now() - start
                }
                [..] => todo!(),
            }
        }
    }

    pub fn dump_code(&mut self) -> () {
        Target::initialize_native(&InitializationConfig::default())
            .expect("Failed to initialize llvm");
        let target = Target::get_first().expect("Could not find target");

        let target_machine = target
            .create_target_machine(
                &TargetMachine::get_default_triple(),
                TargetMachine::get_host_cpu_name()
                    .as_ref()
                    .to_str()
                    .unwrap(),
                TargetMachine::get_host_cpu_features()
                    .as_ref()
                    .to_str()
                    .unwrap(),
                OptimizationLevel::Aggressive,
                inkwell::targets::RelocMode::Default,
                inkwell::targets::CodeModel::Default,
            )
            .expect("Could not make target machine");

        target_machine
            .write_to_file(
                &self.module,
                inkwell::targets::FileType::Assembly,
                &Path::new("./out.s"),
            )
            .unwrap();

        let mut gcc = std::process::Command::new("gcc");
        gcc.args(["-no-pie", "out.s", "-o", "bbvm.out"]);
        if !gcc.status().expect("Failed to run GCC").success() {
            panic!("GCC failed to compile the assembly code");
        }

        std::io::stdout()
            .write_all(
                &std::process::Command::new("./bbvm.out")
                    .output()
                    .expect("Failed to run compiled code")
                    .stdout,
            )
            .unwrap();
    }
}
