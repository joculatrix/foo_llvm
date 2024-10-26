use super::*;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::FloatValue;
use inkwell::AddressSpace;

use crate::parse::Expr;

use std::error::Error;
use std::collections::HashMap;

/// Used to traverse the program AST and generate the LLVM IR.
/// 
/// This struct itself shouldn't be needed externally; only its public non-method
/// [`generate()`] function. The [`new()`] function as well as all of the
/// struct's methods are only used internally.
/// 
/// [`generate()`]:     Self::generate()
/// [`new()`]:          Self::new()
pub struct LlvmGenerator<'a, 'ctx> {
    /// The LLVM Context for the program's module. Used to manage types and
    /// generate the module and builder.
    context: &'ctx Context,
    /// The module the program's content is contained in.
    module: &'a Module<'ctx>,
    /// Handles building of code blocks, functions, and calls.
    builder: &'a Builder<'ctx>,
}

impl<'a, 'ctx> LlvmGenerator<'a, 'ctx> {
    /// Builds a new `LlvmIR`.
    /// 
    /// Takes in a [`Context`] and uses it to build a [`Module`] with name
    /// `module_name`, as well as a [`Builder`].
    /// 
    /// * `context` - The LLVM Context for the program.
    /// * `module_name` - For IR readability.
    fn new(
        context: &'ctx Context,
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>,
    ) -> LlvmGenerator<'a, 'ctx> {
        LlvmGenerator {
            context,
            module,
            builder,
        }
    }

    /// This is the primary function called to execute the IR generation process.
    /// 
    /// Loops through each [`Fn`] or [`Let`] and their `then` values until
    /// reaching the final expression for evaluation.
    /// 
    /// * `root` - The root node of the AST.
    /// 
    /// [`Fn`]:     Expr::Fn
    /// [`Let`]:    Expr::Let
    fn run(&self, root: &Expr) -> Result<(), Box<dyn Error>> {
        let mut vars = HashMap::new();
        let mut e = root;

        let main = self.module.add_function(
            "main",
            self.context.void_type().fn_type(&[], false),
            None
        );
        let main_block = self.context.append_basic_block(main, "main_enter");
        self.builder.position_at_end(main_block);

        loop { // loop through Fn and Let until `e` is some other expression type
            match e {
                // If anyone reading is confused: the `name` field is a tuple of
                // both a string and a locational span; the `name` identifier is
                // being shadowed here to refer to only the string.
                Expr::Fn { name: (name, _), args, body, then, .. } => {
                    // `args` also gets mapped to a span-less variant:
                    let args = args.iter().map(|(name, _)| name);

                    if let Some(_) = self.module.get_function(&name) {
                        return Err(format!("function `{}` already exists", name).into());
                    } else {
                        // create function and add it to the module
                        let arg_types = std::iter::repeat(self.context.f64_type())
                            .take(args.len())
                            .map(|t| t.into())
                            .collect::<Vec<BasicMetadataTypeEnum>>();
                        let r#fn = self.module.add_function(
                            &name,
                            self.context
                                .f64_type()
                                .fn_type(
                                    &arg_types,
                                    false
                                ),
                            None
                        );
                        // set param names
                        r#fn.get_param_iter()
                            .zip(args)
                            .for_each(|(param, arg)| {
                                param.set_name(&arg);
                            }
                        );
                        // generate function body
                        let block = self.context.append_basic_block(
                            r#fn, 
                            &format!("{}_enter", name)
                        );
                        self.builder.position_at_end(block);
    
                        let mut fn_vars = HashMap::new();
                        r#fn.get_param_iter().for_each(|param| {
                            fn_vars.insert(
                                param.get_name().to_str().unwrap().to_owned(),
                                param.into_float_value()
                            );
                        });
    
                        self.builder.build_return(Some(&self.visit_expr(body, &fn_vars)?))?;
                        
                        if r#fn.verify(true) {
                            e = &then;
                            self.builder.position_at_end(main_block);
                        } else {
                            return Err(format!("function `{}` not built properly", name).into());
                        }
                    }
                }
                Expr::Let { name: (name, _), rhs, then, .. } => {
                    vars.insert(name.to_owned(), self.visit_expr(rhs, &vars)?);
                    e = &then;
                }
                _ => {
                    let exp = self.visit_expr(e, &vars)?;
                    
                    // call printf from libc
                    let printf = self.module.add_function(
                        "printf",
                        self.context
                            .i32_type()
                            .fn_type(
                                &[
                                    BasicMetadataTypeEnum::PointerType(
                                        self.context.ptr_type(AddressSpace::default())
                                    ),
                                ],
                                true
                            ),
                        None
                    );
                    let format = self.builder.build_global_string_ptr("%f\n", "fmtstr")?;
                    self.builder.build_call(
                        printf,
                        &[
                            format.as_pointer_value().into(),
                            exp.into()
                        ],
                        "calltmp"
                    )?;
                    break;
                }
            }
        }
        self.builder.build_return(None)?;
        main.verify(true);

        Ok(())
    }

    /// Recursively handles non-[`Fn`] and non-[`Let`] expressions, whether for
    /// let assignment values, function bodies, or the final expression the
    /// program returns.
    /// 
    /// Calls [`visit_call()`] as a helper for function call expressions.
    /// 
    /// [`Fn`]:             Expr::Fn
    /// [`Let`]:            Expr::Let
    /// [`visit_call()`]:   Self::visit_call()
    fn visit_expr(
        &self,
        expr: &Expr,
        vars: &HashMap<String, FloatValue<'ctx>>
    ) -> Result<FloatValue<'ctx>, Box<dyn Error>> {
        match expr {
            Expr::Add(left, right, _) => {
                let left = self.visit_expr(left, vars)?;
                let right = self.visit_expr(right, vars)?;

                Ok(self.builder.build_float_add(left, right, "addtmp")?)
            }
            Expr::Sub(left, right, _) => {
                let left = self.visit_expr(left, vars)?;
                let right = self.visit_expr(right, vars)?;

                Ok(self.builder.build_float_sub(left, right, "subtmp")?)
            }
            Expr::Mul(left, right, _) => {
                let left = self.visit_expr(left, vars)?;
                let right = self.visit_expr(right, vars)?;

                Ok(self.builder.build_float_mul(left, right, "multmp")?)
            }
            Expr::Div(left, right, _) => {
                let left = self.visit_expr(left, vars)?;
                let right = self.visit_expr(right, vars)?;

                Ok(self.builder.build_float_div(left, right, "divtmp")?)
            }
            Expr::Num(val, _) => Ok(self.context.f64_type().const_float(*val)),
            Expr::Var(name, _) => match vars.get(name) {
                Some (val) => Ok(val.to_owned()),
                None => Err(format!("variable `{}` not found in scope", name).into()),
            }
            Expr::Neg(expr, _) => {
                let expr = self.visit_expr(expr, vars)?;
                Ok(self.builder.build_float_neg(expr, "negtmp")?)
            }
            Expr::Call((name, _), args, _) => self.visit_call(name, args, vars),
            _ => panic!()
        }
    }

    /// Helper function for [`visit_expr()`]. Checks that a function call is
    /// valid and, if so, grabs the return value from the call.
    /// 
    /// [`visit_expr()`]:   Self::visit_expr()
    fn visit_call(
        &self,
        name: &String,
        args: &Vec<Expr>,
        vars: &HashMap<String, FloatValue<'ctx>>
    ) -> Result<FloatValue<'ctx>, Box<dyn Error>> {
        match self.module.get_function(name) {
            None => Err(format!("function `{}` not found in scope", name).into()),
            Some(r#fn) => {
                if args.len() != r#fn.get_params().len() {
                    return Err("arguments to function call are incorrect".into());
                }
                let mut argsv = vec![];
                for arg in args {
                    argsv.push(self.visit_expr(arg, vars)?.into());
                }
                match self.builder
                    .build_call(r#fn, &argsv, "calltmp")?
                    .try_as_basic_value()
                    .left()
                {
                    Some(val) => Ok(val.into_float_value()),
                    None => Err("failed to build function call".into()),
                }
            }
        }
    }

    /// This is the function called externally to input the AST [`Expr`] along
    /// with the LLVM `Context`, `Module`, and `Builder` and generate the IR.
    /// 
    /// If there are no errors, this function doesn't return anything useful.
    /// Instead, the module can be used to do further actions with the IR.
    pub fn generate(
        ast: &Expr,
        context: &'ctx Context,
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>
    ) -> Result<(), Box<dyn Error>> {
        let generator = LlvmGenerator::new(context, module, builder);
        generator.run(ast)
    }
}