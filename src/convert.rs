use std::{collections::HashMap, iter::zip, rc::Rc};

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    types::IntType,
    values::{IntValue, PhiValue},
    AddressSpace, IntPredicate,
};

type Label<'a> = (BasicBlock<'a>, BasicBlock<'a>);

pub struct Converter<'a> {
    context: Rc<Context>,
    module: Module<'a>,
    builder: Builder<'a>,
    variables: Vec<Vec<IntValue<'a>>>,
    phis: Vec<(Vec<PhiValue<'a>>, Label<'a>)>,
    mapping: HashMap<Rc<str>, usize>,
    one: IntValue<'a>,
    zero: IntValue<'a>,
    l128: IntType<'a>,
}

impl<'a> Converter<'a> {
    pub fn new(varis: Vec<Rc<str>>) -> Converter<'a> {
        let context = Rc::new(Context::create());
        let module = context.create_module("bbvm");
        let l128 = context.i128_type();
        let one = l128.const_int(1, false);
        let zero = l128.const_zero();
        let main = module.add_function("main", l128.fn_type(&[], false), None);
        let block = context.append_basic_block(main, "entry");
        let builder = context.create_builder();
        builder.position_at_end(block);

        let variables = vec![vec![l128.const_int(0, false); varis.len()]];
        let phis = vec![];
        let mut mapping = HashMap::new();
        for v in varis.iter().enumerate() {
            mapping.insert(*v.1, v.0);
        }
        Converter {
            context,
            module,
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
    pub fn addIncr(&mut self, var: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        let pos = self.mapping[&var];

        vars[pos] = self.builder.build_int_add(vars[pos], self.one, "incr");
    }

    // if var != 0 {
    //   var = var - 1
    // }
    pub fn addDecr(&mut self, var: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        let pos = self.mapping[&var];

        let current = vars[pos];

        let cmp = self.builder.build_int_compare(
            IntPredicate::EQ,
            current,
            self.l128.const_zero(),
            "cmp_to_0",
        );

        let main = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

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
    pub fn addClear(&mut self, var: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        vars[self.mapping[&var]] = self.l128.const_zero();
    }

    // to = from
    pub fn addCopy(&mut self, from: Rc<str>, to: Rc<str>) -> () {
        let vars = self.variables.last_mut().expect("ERROR stack frame empty!");
        vars[self.mapping[&to]] = vars[self.mapping[&from]];
    }

    pub fn addWhile<'b>(&mut self, var: Rc<str>, check: i128) -> () {
        let main = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
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

    pub fn addEnd(&mut self) -> () {
        let (phis, (start, end)) = self
            .phis
            .pop()
            .expect("ERROR: Phis list empty (too many \"end\"s?)");
        self.builder.build_unconditional_branch(start);
        self.builder.position_at_end(end);
        let vars = self.variables.pop().expect("ERROR stack frame empty!");
        for (phi, var) in zip(phis, vars) {
            phi.add_incoming(&[(&var, self.builder.get_insert_block().unwrap())]);
        }
        self.variables.push(
            phis.iter()
                .map(|phi| phi.as_basic_value().into_int_value())
                .collect(),
        );
    }
    pub fn addEOF(&mut self) -> () {
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
        for var in self.mapping.iter() {
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
        //self.module.verify().unwrap();
    }
}
