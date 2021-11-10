use std::{collections::HashMap, iter::zip, path::Path, rc::Rc};

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    execution_engine::JitFunction,
    module::Module,
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
    variables: Vec<Vec<IntValue<'a>>>,
    phis: Vec<(Vec<PhiValue<'a>>, Label<'a>)>,
    mapping: HashMap<Rc<str>, usize>,
    one: IntValue<'a>,
    zero: IntValue<'a>,
    l128: IntType<'a>,
}

impl<'a> Converter<'a> {
    pub fn new(varis: Vec<Rc<str>>, context: &'a Context) -> Converter<'a> {
        let module: Module<'a> = context.create_module("bbvm");
        let l128 = context.i128_type();
        let one = l128.const_int(1, false);
        let zero = l128.const_zero();
        let main = module.add_function("main", context.void_type().fn_type(&[], false), None);
        let block = context.append_basic_block(main, "entry");
        let builder = context.create_builder();
        builder.position_at_end(block);

        let variables = vec![vec![l128.const_int(0, false); varis.len()]];
        let phis = vec![];
        let mut mapping = HashMap::new();
        for v in varis.iter().enumerate() {
            mapping.insert(v.1.clone(), v.0);
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
            l128,
        }
    }

    // var = var + 1
    pub fn add_incr(&mut self, var: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        let pos = self.mapping[&var];

        vars[pos] = self.builder.build_int_add(vars[pos], self.one, "incr");
    }

    // if var != 0 {
    //   var = var - 1
    // }
    pub fn add_decr(&mut self, var: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        let pos = self.mapping[&var];

        let current = vars[pos];

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
        let res = self.builder.build_phi(self.l128, "result");
        res.add_incoming(&[
            (&current, self.builder.get_insert_block().unwrap()),
            (&new_var, no_skip),
        ]);

        vars[pos] = res.as_basic_value().into_int_value();
    }

    // var = 0
    pub fn add_clear(&mut self, var: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        vars[self.mapping[&var]] = self.zero;
    }

    // to = from
    pub fn add_copy(&mut self, from: Rc<str>, to: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        vars[self.mapping[&to]] = vars[self.mapping[&from]];
    }

    pub fn add_while<'b>(&'b mut self, var: Rc<str>, check: i128) -> () {
        let main = self.main;
        let lop = self.context.append_basic_block(main, "loop");
        self.builder.build_unconditional_branch(lop);
        self.builder.position_at_end(lop);

        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        let phis = vars
            .iter()
            .map(|var| {
                let rf = self.builder.build_phi(self.l128, "whilePhi");
                rf.add_incoming(&[(var, self.builder.get_insert_block().unwrap())]);
                rf
            })
            .collect::<Vec<PhiValue>>();

        let cmp = self.builder.build_int_compare(
            IntPredicate::EQ,
            phis[self.mapping[var.as_ref()]]
                .as_basic_value()
                .into_int_value(),
            self.l128.const_int(check as u64, false),
            "exitCondition",
        );
        let inner_loop = self.context.append_basic_block(main, "innerLoop");
        let exit = self.context.append_basic_block(main, "loopExit");
        self.builder.build_conditional_branch(cmp, exit, inner_loop);
        self.builder.position_at_end(inner_loop);

        self.phis.push((phis, (lop, exit)));
    }

    pub fn add_end(&mut self) -> () {
        let (phis, (start, end)) = self
            .phis
            .pop()
            .expect("ERROR: Phis list empty (too many \"end\"s?)");
        self.builder.build_unconditional_branch(start);
        self.builder.position_at_end(end);
        let vars = self.variables.pop().expect("ERROR stack frame empty!");
        for (phi, var) in zip(&phis, vars) {
            phi.add_incoming(&[(&var, self.builder.get_insert_block().unwrap())]);
        }
        self.variables.push(
            phis.iter()
                .map(|phi| phi.as_basic_value().into_int_value())
                .collect(),
        );
    }
    pub fn add_eof<'b>(&'b mut self) -> () {
        if self.phis.len() > 0 {
            panic!("Too many opening while loops!")
        }
        if self.variables.len() > 1 {
            panic!("ERROR stack frame NOT empty!")
        }
        let fun = self.context.void_type().fn_type(
            &[
                self.context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .into(),
                self.context.i64_type().into(),
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
                &[
                    fmt.as_pointer_value().into(),
                    self.variables.pop().unwrap()[*var.1].into(),
                ],
                "printf",
            );
        }

        self.builder.build_return(None);
        self.module.verify().unwrap();
    }

    pub fn run(&mut self) -> () {
        let execution_engine = self
            .module
            .create_jit_execution_engine(OptimizationLevel::Aggressive)
            .expect("Unable to create execution engine");
        self.module.print_to_stderr();
        unsafe {
            let main: JitFunction<unsafe extern "C" fn() -> ()> = execution_engine
                .get_function("main")
                .expect("Unable to load function");
            main.call();
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
                inkwell::targets::FileType::Object,
                &Path::new("./out.o"),
            )
            .unwrap();

        target_machine
            .write_to_file(
                &self.module,
                inkwell::targets::FileType::Assembly,
                &Path::new("./out.asm"),
            )
            .unwrap();
    }
}
