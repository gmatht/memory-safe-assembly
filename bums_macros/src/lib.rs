extern crate proc_macro;
use parse::{Parse, ParseStream};
use proc_macro::{Span, TokenStream};
#[allow(unused_imports)]
use proc_macro_error::*;
use punctuated::Punctuated;
use quote::quote;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader};
use syn::*;
use z3::{Config, Context};
use z3::ast::Ast;

use bums::common::*;

#[derive(Debug)]
struct CallColon {
    item_fn: Signature,
    _end_token: Token![;],
}

impl Parse for CallColon {
    fn parse(input: ParseStream) -> Result<Self> {
        return Ok(Self {
            item_fn: input.parse()?,
            _end_token: input.parse()?,
        });
    }
}

#[derive(Debug)]
struct AttributeList {
    filename: LitStr,
    _separator: Option<Token![,]>,
    argument_list: Punctuated<Expr, Token![,]>,
}

impl Parse for AttributeList {
    fn parse(input: ParseStream) -> Result<Self> {
        return Ok(Self {
            filename: input.parse()?,
            _separator: input.parse()?,
            argument_list: punctuated::Punctuated::<Expr, Token![,]>::parse_terminated(input)?,
        });
    }
}

fn calculate_size_of(ty: String) -> usize {
    match ty.as_str() {
        "i8" => std::mem::size_of::<i8>(),
        "i16" => std::mem::size_of::<i16>(),
        "i32" => std::mem::size_of::<i32>(),
        "i64" => std::mem::size_of::<i64>(),
        "u8" => std::mem::size_of::<u8>(),
        "u16" => std::mem::size_of::<u16>(),
        "u32" => std::mem::size_of::<u32>(),
        "u64" => std::mem::size_of::<u64>(),
        "u128" => std::mem::size_of::<u128>(),
        "usize" => std::mem::size_of::<u128>(),
        "isize" => std::mem::size_of::<isize>(),
        p => {
            if let Ok(v) = p.parse::<usize>() {
                return v;
            } else {
                todo!("size of undefined type")
            }
        }
    }
}

fn calculate_size_of_array(a: &TypeArray) -> usize {
    let elem: String;
    let len;
    match &*a.elem {
        Type::Path(b) => {
            elem = b.path.segments[0].ident.to_string();
        }
        _ => todo!("calculate size of array that is not given using path"),
    }
    match &a.len {
        Expr::Lit(b) => match &b.lit {
            Lit::Int(i) => {
                len = i
                    .token()
                    .to_string()
                    .parse::<usize>()
                    .expect("calculate_size_array");
            }
            _ => todo!("size of array1"),
        },
        _ => todo!("size of array2"),
    }
    return calculate_size_of(elem) * len;
}

fn calculate_type_of_array_ptr(a: &TypeArray) -> String {
    let elem: String;
    match &*a.elem {
        Type::Path(b) => {
            elem = b.path.segments[0].ident.to_string();
        }
        Type::Array(r) => {
            let inner;
            let size;
            match &*r.elem {
                Type::Path(p) => {
                    inner = p.path.segments[0].ident.to_string();
                }
                _ => todo!("calculate type of array not with path {:?}", r),
            }
            match &r.len {
                Expr::Lit(l) => match &l.lit {
                    Lit::Int(i) => size = i.to_string(),
                    _ => todo!(),
                },
                _ => todo!(),
            }
            elem = "[".to_owned() + &inner + &";" + &size + "]";
        }
        _ => todo!("calculate type of array not with path {:?}", a),
    }
    return elem;
}

fn calculate_type_of_slice_ptr(a: &TypeSlice) -> String {
    let elem: String;
    match &*a.elem {
        Type::Path(b) => {
            elem = b.path.segments[0].ident.to_string();
        }
        _ => todo!("calculate type of slice"),
    }
    return elem;
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn binary_to_abstract_expression(input: &ExprBinary) -> AbstractExpression {
    let left_expr = syn_expr_to_abstract_expression(&input.left);
    let right_expr = syn_expr_to_abstract_expression(&input.right);

    match input.op {
        BinOp::Add(_) => return generate_expression("+", left_expr, right_expr),
        BinOp::Sub(_) => return generate_expression("-", left_expr, right_expr),
        BinOp::Div(_) => return generate_expression("/", left_expr, right_expr),
        BinOp::Mul(_) => return generate_expression("*", left_expr, right_expr),
        BinOp::Rem(_) => return generate_expression("%", left_expr, right_expr),
        _ => todo!("expression binary to abstract {:?}", input.op),
    }
}

fn binary_to_abstract_comparison(input: &ExprBinary) -> AbstractComparison {
    let left_expr = syn_expr_to_abstract_expression(&input.left);
    let right_expr = syn_expr_to_abstract_expression(&input.right);

    match input.op {
        BinOp::Eq(_) => return generate_comparison("==", left_expr, right_expr),
        BinOp::Lt(_) => return generate_comparison("<", left_expr, right_expr),
        BinOp::Le(_) => return generate_comparison("<=", left_expr, right_expr),
        BinOp::Ne(_) => return generate_comparison("!=", left_expr, right_expr),
        BinOp::Ge(_) => return generate_comparison(">=", left_expr, right_expr),
        BinOp::Gt(_) => return generate_comparison(">", left_expr, right_expr),
        _ => todo!("comparison conversion"),
    }
}
fn unary_to_abstract_expression(input: &ExprUnary) -> AbstractExpression {
    let expr = syn_expr_to_abstract_expression(&input.expr);
    match input.op {
        UnOp::Not(_) => return generate_expression("!", expr, AbstractExpression::Empty),
        UnOp::Neg(_) => return generate_expression("-", expr, AbstractExpression::Empty),
        _ => todo!("unary conversion"),
    }
}

fn syn_expr_to_abstract_expression(input: &Expr) -> AbstractExpression {
    match &input {
        Expr::Lit(l) => match &l.lit {
            Lit::Str(s) => return AbstractExpression::Abstract(s.value()),
            Lit::Int(i) => {
                return AbstractExpression::Immediate(
                    i.base10_parse::<i64>().expect("undefined integer"),
                )
            }
            _ => todo!("Input Literal type"),
        },
        Expr::Binary(b) => return binary_to_abstract_expression(b),
        Expr::Unary(b) => return unary_to_abstract_expression(b),
        Expr::MethodCall(c) => {
            let mut var_name: String;
            match *c.receiver.clone() {
                Expr::Path(a) => {
                    var_name = a.path.segments[0].ident.to_string();
                }
                _ => todo!("method matching to get name"),
            }
            match c.method.to_string().as_str() {
                "len" => {
                    var_name = var_name + "_len";
                }
                "as_ptr" => {
                    var_name = var_name + "_as_ptr";
                }
                "as_mut_ptr" => {
                    var_name = var_name + "_as_mut_ptr";
                }
                _ => todo!("method matching"),
            };
            return AbstractExpression::Abstract(var_name);
        }
        Expr::Path(p) => {
            return AbstractExpression::Abstract(p.path.segments[0].ident.to_string());
        }
        Expr::Field(f) => {
            let base = match &*f.base {
                Expr::Path(p) => p.clone().path.segments[0].ident.to_string(),
                _ => todo!("field processing {:?}", f.base),
            };
            let index = match &f.member {
                Member::Named(n) => n.to_string(),
                Member::Unnamed(n) => n.index.to_string(),
            };
            let new_name = base.to_owned() + "_field" + &index;
            return AbstractExpression::Abstract(new_name);
        }
        _ => todo!("Input type {:?}", input),
    }
}

fn tuple_to_struct(name: String, tuple: TypeTuple) -> ItemStruct {
    let span = Span::call_site().into();

    // Creating fields for the struct
    let mut fields: syn::punctuated::Punctuated<Field, token::Comma> = Punctuated::new();
    for (index, expr) in tuple.elems.iter().enumerate() {
        let field_ident = syn::Ident::new(&format!("{}_field{}", name, index), span);
        let field: Field = parse_quote! {#field_ident: #expr};
        fields.push(field.clone());
    }

    let struct_name = syn::Ident::new(&(name + "_struct"), span);
    parse_quote! { #[repr(C)] struct #struct_name { #fields }}
}

// ATTRIBUTE ON EXTERN BLOCK
#[proc_macro_attribute]
#[proc_macro_error]
pub fn check_mem_safe(attr: TokenStream, item: TokenStream) -> TokenStream {
    let vars = parse_macro_input!(item as CallColon);
    let mut attributes = parse_macro_input!(attr as AttributeList);
    let fn_name = &vars.item_fn.ident;
    let output = &vars.item_fn.output;

    let mut invariants: Vec<AbstractComparison> = Vec::new();
    let mut asserts = quote! {};
    if let Some(Expr::Array(a)) = attributes.argument_list.last() {
        for e in &a.elems {
            if let Expr::Binary(b) = e {
                invariants.push(binary_to_abstract_comparison(b));
                asserts = quote! { #asserts assert!(#e);};
            } else {
                emit_call_site_error!("Cannot define an invariant that is not a binary expression");
            }
        }
        attributes.argument_list.pop();
    }

    //get args from function call to pass to invocation
    let mut arguments_to_memory_safe_regions = Vec::new();
    let mut input_sizes = HashMap::new();
    let mut pointer_sizes = HashMap::new();
    let mut input_types = HashMap::new();
    let mut input_expressions = HashMap::new();
    let mut arguments_to_pass: Punctuated<_, _> = Punctuated::new();
    let mut new_structs = HashMap::new();
    // if caller did not specify arguments in macro, grab names from function call
    if attributes.argument_list.is_empty() {
        for i in &vars.item_fn.inputs {
            arguments_to_memory_safe_regions.push(i.clone());
            match i {
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(a) => {
                        let s = a.ident.clone();
                        let mut q = Punctuated::new();
                        q.push(PathSegment {
                            ident: s,
                            arguments: PathArguments::None,
                        });
                        let w = Expr::Path(ExprPath {
                            attrs: Vec::new(),
                            qself: None,
                            path: Path {
                                leading_colon: None,
                                segments: q,
                            },
                        });
                        arguments_to_pass.push(w);
                    }
                    _ => todo!("non-ident name"),
                },
                _ => todo!("non-typed name"),
            }
        }
    } else {
        for i in &vars.item_fn.inputs {
            match i {
                FnArg::Typed(pat_type) => {
                    // get name
                    let name;
                    match &*pat_type.pat {
                        Pat::Ident(b) => {
                            name = b.ident.clone().to_string();
                        }
                        _ => todo!("non-ident typed name in inputs"),
                    }
                    let ty = &*pat_type.ty;
                    input_types.insert(name.clone(), ty);
                    match ty {
                        Type::Array(a) => {
                            let ty = calculate_type_of_array_ptr(a);
                            let size = calculate_size_of_array(a);
                            input_sizes.insert(name.clone(), size * 2);
                            pointer_sizes.insert(name, ty);
                        }
                        Type::Reference(a) => match &*a.elem {
                            Type::Array(b) => {
                                let ty = calculate_type_of_array_ptr(b);
                                let size = calculate_size_of_array(b);
                                pointer_sizes.insert(name.clone(), ty);
                                input_sizes.insert(name, size * 2);
                            }
                            Type::Slice(b) => {
                                let ty = calculate_type_of_slice_ptr(b);
                                pointer_sizes.insert(name, ty);
                            }
                            Type::Tuple(t) => {
                                let mut size = 0;
                                new_structs
                                    .insert(name.clone(), tuple_to_struct(name.clone(), t.clone()));
                                for e in &t.elems {
                                    match e {
                                        Type::Array(a) => {
                                            size = size + calculate_size_of_array(&a);
                                        }
                                        Type::Path(p) => {
                                            for i in &p.path.segments {
                                                match i.ident.to_string().as_str() {
                                                    "usize" => {
                                                        size = size + std::mem::size_of::<usize>();
                                                    }
                                                    "u32" => {
                                                        size = size + std::mem::size_of::<u32>();
                                                    }
                                                    _ => todo!("path size"),
                                                }
                                            }
                                        }
                                        _ => todo!("element list type"),
                                    }
                                }
                                input_sizes.insert(name.clone(), size);
                            }
                            _ => todo!("Input Reference Type"),
                        },
                        Type::Path(p) => {
                            let ty = p.path.segments[0].ident.to_string();
                            let size = calculate_size_of(ty);
                            input_sizes.insert(name, size * 2);
                        }
                        _ => todo!("Standard Input type {:?}", ty),
                    }
                }
                _ => todo!("Untyped args"),
            }
        }
        for i in &attributes.argument_list {
            if let Expr::Cast(c) = i {
                let struct_name;
                if let Expr::Path(p) = &*c.expr {
                    struct_name = p.path.segments[0].ident.to_string();
                } else {
                    struct_name = "struct".to_string();
                }

                let mut fields = quote! {};
                let mut i = 0;
                let struct_ident = Ident::new(&struct_name, proc_macro2::Span::call_site().into());
                for f in &new_structs
                    .get(&struct_name)
                    .expect("Need established struct")
                    .fields
                {
                    let fieldname = f.ident.clone().expect("Need field name");
                    let lit: Lit = Lit::new(proc_macro2::Literal::usize_unsuffixed(i));
                    fields = quote! { #fields #fieldname: #struct_ident.#lit, };
                    i = i + 1;
                }

                let real_struct_name = Ident::new(
                    &(struct_name + "_struct"),
                    proc_macro2::Span::call_site().into(),
                );
                arguments_to_pass
                    .push(parse_quote! {&#real_struct_name { #fields } as *const #real_struct_name})
            } else {
                arguments_to_pass.push(i.clone());
            }
        }
    }

    // extract name of function being invoked to pass to invocation
    let mut q = Punctuated::new();
    q.push(PathSegment {
        ident: fn_name.clone(),
        arguments: PathArguments::None,
    });

    let invocation: ExprCall = ExprCall {
        attrs: vec![],
        func: Box::new(Expr::Path(ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: Path {
                leading_colon: None,
                segments: q,
            },
        })),
        paren_token: Default::default(),
        args: arguments_to_pass,
    };

    let mut extern_fn = vars.item_fn.clone();
    extern_fn.ident = fn_name.clone();
    if !attributes.argument_list.is_empty() {
        let mut new_args: Punctuated<FnArg, Token![,]> = Punctuated::new();
        let mut span = proc_macro2::Span::call_site();
        for i in attributes.argument_list {
            match i {
                Expr::MethodCall(a) => {
                    let var_name: String;
                    match *a.receiver {
                        Expr::Path(a) => {
                            var_name = a.path.segments[0].ident.to_string();
                            span = a.path.segments[0].ident.span();
                        }
                        Expr::MethodCall(a) => {
                            todo!("handle nested method calls {:?}", a)
                        }
                        _ => todo!("non-path receiver of a method call {:?}", a),
                    };
                    match a.method.to_string().as_str() {
                        "len" => {
                            let n = Ident::new(&(var_name + "_len"), span.into());
                            new_args.push(parse_quote! {#n: usize});
                        }
                        "as_ptr" => {
                            let n = Ident::new(&(var_name.clone() + "_as_ptr"), span.into());
                            if let Some(size) = pointer_sizes.get(&var_name) {
                                match size.as_str() {
                                    "u8" => new_args.push(parse_quote! {#n: *const u8}),
                                    "u16" => new_args.push(parse_quote! {#n: *const u16}),
                                    "i16" => new_args.push(parse_quote! {#n: *const i16}),
                                    "u32" => new_args.push(parse_quote! {#n: *const u32}),
                                    "u64" => new_args.push(parse_quote! {#n: *const u64}),
                                    "u128" => new_args.push(parse_quote! {#n: *const u128}),
                                    "usize" => new_args.push(parse_quote! {#n: *const usize}),
                                    _ => todo!("ptr array size 1"),
                                }
                            } else {
                                new_args.push(parse_quote! {#n: *const usize});
                            }
                        }
                        "as_mut_ptr" => {
                            let n = Ident::new(&(var_name.clone() + "_as_mut_ptr"), span.into());
                            if let Some(size) = pointer_sizes.get(&var_name) {
                                match size.as_str() {
                                    "u8" => new_args.push(parse_quote! {#n: *mut u8}),
                                    "u16" => new_args.push(parse_quote! {#n: *mut u16}),
                                    "u32" => new_args.push(parse_quote! {#n: *mut u32}),
                                    "u64" => new_args.push(parse_quote! {#n: *mut u64}),
                                    "u128" => new_args.push(parse_quote! {#n: *mut u128}),
                                    "[u64;5]" => new_args.push(parse_quote! {#n: *mut [u64;5]}), // TODO: automate
                                    "usize" => new_args.push(parse_quote! {#n: *mut usize}),
                                    _ => todo!("ptr array size 2"),
                                }
                            } else {
                                new_args.push(parse_quote! {#n: *mut usize});
                            }
                        }
                        _ => todo!("method call in new args"),
                    };
                }
                Expr::Reference(_) => {
                    // TODO include a name in var name for uniqueness
                    new_args.push(parse_quote! {_ : u32});
                }
                Expr::Path(ref a) => {
                    let var_name = a.path.segments[0].ident.to_string();
                    if let Some(ty) = input_types.get(&var_name) {
                        new_args.push(parse_quote! {#i: #ty});
                    }
                }
                Expr::Binary(ref b) => {
                    let exp = quote! {#b}.to_string();
                    let name = "expr_".to_owned() + &calculate_hash(&exp).to_string();
                    input_expressions.insert(name.clone(), i);
                    let n = Ident::new(&name, span.into());
                    new_args.push(parse_quote! {#n : usize});
                }
                Expr::Unary(ref b) => {
                    let exp = quote! {#b}.to_string();
                    let name = "expr_".to_owned() + &calculate_hash(&exp).to_string();
                    input_expressions.insert(name.clone(), i);
                    let n = Ident::new(&name, span.into());
                    new_args.push(parse_quote! {#n : isize});
                }
                Expr::Lit(ref b) => {
                    let exp = quote! {#b}.to_string();
                    let name = "expr_".to_owned() + &calculate_hash(&exp).to_string();
                    input_expressions.insert(name.clone(), i);
                    let n = Ident::new(&name, span.into());
                    new_args.push(parse_quote! {#n : usize});
                }
                Expr::Cast(c) => {
                    let var_name: String;
                    match &*c.expr {
                        Expr::Reference(r) => match &*r.expr {
                            Expr::Path(p) => {
                                var_name = p.path.segments[0].ident.to_string();
                            }
                            _ => todo!("name of cast ref expr types {:?}", r),
                        },
                        Expr::Path(p) => {
                            var_name = p.path.segments[0].ident.to_string();
                        }
                        _ => todo!("name of cast expr types {:?}", c.expr),
                    }

                    let n = Ident::new(&(var_name.clone() + "_as_mut_ptr"), span.into());
                    let ty = c.ty.clone();
                    if let Type::Ptr(p) = *ty.clone() {
                        if let Type::Infer(_) = *p.elem {
                            let struct_name =
                                Ident::new(&(var_name.clone() + "_struct"), span.into());
                            new_args.push(parse_quote! {#n : *const #struct_name});
                        } else {
                            new_args.push(parse_quote! {#n : #ty});
                        }
                    } else {
                        new_args.push(parse_quote! {#n : #ty});
                    }
                }
                Expr::Field(f) => {
                    let var_name: String;
                    match &*f.base {
                        Expr::Path(p) => {
                            var_name = p.path.segments[0].ident.to_string();
                        }
                        Expr::MethodCall(m) => {
                            match &*m.receiver {
                                Expr::Path(a) => {
                                    var_name = a.path.segments[0].ident.to_string();
                                    span = a.path.segments[0].ident.span();
                                }
                                Expr::MethodCall(a) => {
                                    todo!("handle nested method calls {:?}", a)
                                }
                                _ => todo!("non-path receiver of a method call {:?}", m),
                            };
                        }
                        _ => todo!("name of field expr types {:?}", f.base),
                    }
                    match *f.base {
                        Expr::MethodCall(ref m) => match m.method.to_string().as_str() {
                            "as_ptr_range" => {
                                let pointer_size = pointer_sizes
                                    .get(&var_name)
                                    .expect("can't find size of  slice");
                                let pointer_type = Ident::new(&pointer_size.clone(), span.into());
                                match f.member {
                                    syn::Member::Named(name) => match name.to_string().as_str() {
                                        "end" => {
                                            let n = Ident::new(
                                                &(var_name.clone() + "_end_ptr_range"),
                                                span.into(),
                                            );
                                            new_args.push(parse_quote! {#n : *const #pointer_type});
                                        }
                                        _ => todo!("more subfields of range"),
                                    },
                                    syn::Member::Unnamed(i) => match i.index {
                                        0 => {
                                            let n = Ident::new(
                                                &(var_name.clone() + "_start_ptr_range"),
                                                span.into(),
                                            );
                                            new_args.push(parse_quote! {#n : _});
                                        }
                                        1 => {
                                            let n = Ident::new(
                                                &(var_name.clone() + "_end_ptr_range"),
                                                span.into(),
                                            );
                                            new_args.push(parse_quote! {#n : _});
                                        }
                                        _ => todo!("irrelevant for this type"),
                                    },
                                }
                            }
                            _ => todo!("match on fields of the results of more methods"),
                        },
                        _ => todo!("match on fields of more methods"),
                    }
                }
                _ => todo!("Arg list type {:?}", i),
            }
        }
        for a in &new_args {
            arguments_to_memory_safe_regions.push(a.clone());
        }
        extern_fn = parse_quote! {fn #fn_name(#new_args)};
    }

    let mut struct_decs = quote! {};
    for i in new_structs.values() {
        struct_decs = quote! {

            #struct_decs

            #i;
        };
    }

    let original_fn_call = vars.item_fn.clone();
    let unsafe_block: Stmt = parse_quote! {
        #original_fn_call {

            #asserts;

            #struct_decs;

            extern "C" {
                #extern_fn #output;
            }
            unsafe {
                return #invocation;
            }
        }
    };

    let token_stream = quote!(#unsafe_block).into();

    // compile file
    // make this path
    let filename = attributes.filename.value();
    let assembly_file: std::path::PathBuf =
        [std::env::var("OUT_DIR").expect("OUT_DIR"), filename.clone()]
            .iter()
            .collect();
    let res = File::open(assembly_file);
    let file: File;
    match res {
        Ok(opened) => {
            file = opened;
        }
        Err(error) => {
            // make more specific using span
            abort_call_site!(error);
        }
    };

    let reader = BufReader::new(file);
    let mut program = Vec::new();
    for line in reader.lines() {
        program.push(line.unwrap_or(String::from("")));
    }

    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let mut engine = bums::engine::ExecutionEngine::new(program, &ctx);

    // add memory safe regions
    for i in 0..arguments_to_memory_safe_regions.len() {
        let name;

        let a = &arguments_to_memory_safe_regions[i];
        match a {
            FnArg::Typed(pat_type) => {
                // get name
                match &*pat_type.pat {
                    Pat::Ident(b) => {
                        name = b.ident.clone().to_string();
                    }
                    Pat::Lit(l) => match &l.lit {
                        Lit::Str(s) => name = s.value(),
                        Lit::Int(i) => name = i.base10_digits().to_string(),
                        _ => todo!("Regions literal pattern {:?}", l),
                    },
                    _ => todo!("Regions pattern pattern"),
                }
                //get type to get size
                match &*pat_type.ty {
                    Type::Path(_) => {
                        if let Some(binary) = input_expressions.get(&name).clone() {
                            match binary {
                                Expr::Binary(b) => {
                                    engine.add_abstract_expression_from(
                                        i,
                                        binary_to_abstract_expression(&b),
                                    );
                                }
                                _ => {
                                    engine.add_abstract_expression_from(
                                        i,
                                        syn_expr_to_abstract_expression(binary),
                                    );
                                }
                            }
                        } else {
                            engine.add_abstract_from(i, name.clone());
                        }
                    }
                    Type::Array(a) => {
                        let size = calculate_size_of_array(a);
                        engine.add_abstract_from(i, name.clone());
                        engine.add_region(
                            RegionType::RW,
                            name.clone(),
                            AbstractExpression::Immediate(size as i64),
                        );
                    }
                    Type::Ptr(a) => {
                        // load pointer into register
                        engine.add_abstract_from(i, name.clone());

                        //derive memory safe region based on length
                        let no_mut_name = name.strip_suffix("_as_mut_ptr").unwrap_or(&name);
                        let no_range = name.strip_suffix("_end_ptr_range").unwrap_or(&no_mut_name);
                        let no_suffix = no_range.strip_suffix("_as_ptr").unwrap_or(no_range);

                        match &*a.elem {
                            Type::Path(p) => {
                                // if pointer to a macro-defined struct
                                if p.path.segments[0].ident.to_string().contains("struct") {
                                    // add the whole region covered by the tuple
                                    if let Some(bound) = input_sizes.get(no_suffix) {
                                        if a.mutability.is_some() {
                                            engine.add_region(
                                                RegionType::WRITE,
                                                name.clone(),
                                                AbstractExpression::Immediate(*bound as i64),
                                            );
                                        } else {
                                            engine.add_region(
                                                RegionType::READ,
                                                name.clone(),
                                                AbstractExpression::Immediate(bound.clone() as i64),
                                            );
                                        }
                                    }

                                    let s = new_structs
                                        .get(no_suffix)
                                        .expect("Need well defined struct");
                                    let mut i = 0;
                                    let mut index = 0;
                                    for e in &s.fields {
                                        match e.ty.clone() {
                                            Type::Path(p) => {
                                                if p.path.segments.len() == 1 {
                                                    let abs = p.path.segments[0].ident.to_string();
                                                    match abs.as_str() {
                                                        "usize" => {
                                                            let new_name = e.ident.clone().expect(
                                                                "need name of variable to input",
                                                            );
                                                            engine.add_abstract_to_memory(
                                                                name.clone(),
                                                                index,
                                                                AbstractExpression::Abstract(
                                                                    new_name.to_string(),
                                                                ),
                                                            );
                                                        }
                                                        "u32" => {
                                                            let new_name = e.ident.clone().expect(
                                                                "need name of variable to input",
                                                            );
                                                            engine.add_abstract_to_memory(
                                                                name.clone(),
                                                                index,
                                                                AbstractExpression::Abstract(
                                                                    new_name.to_string(),
                                                                ),
                                                            );
                                                        }
                                                        _ => todo!("tuple abstracts"),
                                                    }
                                                }
                                            }
                                            Type::Array(a) => {
                                                if let Some(_) = input_sizes.get(no_suffix) {
                                                    index = index
                                                        + ((calculate_size_of_array(&a)) as i64);
                                                }
                                            }
                                            _ => todo!("unsupported tuple type {:?}", e),
                                        }
                                        i = i + 1;
                                    }
                                    continue;
                                }

                                // if pointing to end of array
                                if name.contains("_end_ptr_range") {
                                    // add the whole region covered by the tuple
                                    let bound = no_suffix.to_owned() + "_len";
                                    let pointer_name = no_suffix.to_owned() + "_as_ptr";

                                    engine.add_region(
                                        RegionType::READ,
                                        pointer_name.clone(),
                                        AbstractExpression::Abstract(bound.clone()),
                                    );

                                    //overwrite
                                    engine.add_abstract_expression_from(
                                        i,
                                        generate_expression(
                                            "+",
                                            AbstractExpression::Abstract(pointer_name),
                                            AbstractExpression::Abstract(bound),
                                        ),
                                    );

                                    continue;
                                }

                                // if pointing to an array defined as a function param, no abstract length
                                if let Some(bound) = input_sizes.get(no_suffix) {
                                    if a.mutability.is_some() {
                                        engine.add_region(
                                            RegionType::WRITE,
                                            name.clone(),
                                            AbstractExpression::Immediate(*bound as i64),
                                        );
                                    } else {
                                        engine.add_region(
                                            RegionType::READ,
                                            name.clone(),
                                            AbstractExpression::Immediate(bound.clone() as i64),
                                        );
                                    }
                                    continue;
                                }

                                let bound = no_suffix.to_owned() + "_len";
                                if a.mutability.is_some() {
                                    engine.add_region(
                                        RegionType::WRITE,
                                        name.clone(),
                                        AbstractExpression::Abstract(bound),
                                    );
                                } else {
                                    engine.add_region(
                                        RegionType::READ,
                                        name.clone(),
                                        AbstractExpression::Abstract(bound),
                                    );
                                }
                            }
                            _ => todo!("unsupported pointer type to pass to asm {:?}", a.elem),
                        }
                    }
                    _ => todo!("yet unsupported type: {:?}", pat_type.ty),
                }
            }
            _ => todo!("lib"),
        }
    }

    for i in invariants {
        engine.add_invariant(i);
    }
    let label = vars.item_fn.ident.to_string();
    let res = engine.start(label);

    match res {
        Ok(_) => return token_stream,
        Err(error) => {
            #[cfg(not(debug_assertions))]
            emit_call_site_error!(error);

            #[cfg(debug_assertions)]
            emit_call_site_warning!(error);
            return token_stream;
        }
    };
}

// New attribute for x86 conservative checker
#[proc_macro_attribute]
#[proc_macro_error]
pub fn check_mem_safe_x86(attr: TokenStream, item: TokenStream) -> TokenStream {
    // For MVP reuse most of the logic from check_mem_safe but only do the minimal engine invocation
    let vars = parse_macro_input!(item as CallColon);
    let mut attributes = parse_macro_input!(attr as AttributeList);
    let _fn_name = &vars.item_fn.ident;
    let output = &vars.item_fn.output;

    // minimal: collect asserts if provided
    let mut asserts = quote! {};
    if let Some(Expr::Array(a)) = attributes.argument_list.last() {
        for e in &a.elems {
            asserts = quote! { #asserts assert!(#e); };
        }
        attributes.argument_list.pop();
    }

    // Build extern fn and unsafe invocation like above
    let original_fn_call = vars.item_fn.clone();
    let unsafe_block: Stmt = parse_quote! {
        #original_fn_call {
            #asserts;
            extern "C" {
                #original_fn_call #output;
            }
            unsafe { return #original_fn_call; }
        }
    };

    let token_stream = quote!(#unsafe_block).into();

    // open assembly file
    let filename = attributes.filename.value();
    let assembly_file: std::path::PathBuf =
        [std::env::var("OUT_DIR").expect("OUT_DIR"), filename.clone()]
            .iter()
            .collect();
    let res = File::open(assembly_file);
    let file: File;
    match res {
        Ok(opened) => file = opened,
        Err(error) => abort_call_site!(error),
    }

    let reader = BufReader::new(file);
    let mut program = Vec::new();
    for line in reader.lines() {
        program.push(line.unwrap_or(String::from("")));
    }

    // create z3 context and call x86 engine
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    // write program into a temporary file in OUT_DIR and invoke ExecutionEngineX86::from_asm_file
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let temp_path = std::path::Path::new(&out_dir).join(filename.clone());
    // file already exists in OUT_DIR in normal usage

    // call engine
    let mut engine =
        bums::x86_64::ExecutionEngineX86::from_asm_file(temp_path.to_str().unwrap(), &ctx);

    // Minimal: add memory regions and set initial register abstracts for function arguments.
    // Map first six integer/pointer args to registers (SysV-like): rdi, rsi, rdx, rcx, r8, r9
    let arg_regs = vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
    let mut arg_index = 0usize;
    for input in &vars.item_fn.inputs {
        if let FnArg::Typed(pat_type) = input {
            // get name
            let name = match &*pat_type.pat {
                Pat::Ident(id) => id.ident.to_string(),
                _ => continue,
            };

            // determine size in bytes conservatively
            let mut size_bytes: Option<i64> = None;
            match &*pat_type.ty {
                Type::Array(a) => {
                    let s = calculate_size_of_array(a) as i64;
                    size_bytes = Some(s);
                }
                Type::Reference(a) => match &*a.elem {
                    Type::Array(b) => {
                        let s = calculate_size_of_array(b) as i64;
                        size_bytes = Some(s);
                    }
                    Type::Path(p) => {
                        let ty = p.path.segments[0].ident.to_string();
                        let s = calculate_size_of(ty) as i64;
                        size_bytes = Some(s);
                    }
                    _ => {}
                },
                Type::Path(p) => {
                    let ty = p.path.segments[0].ident.to_string();
                    let s = calculate_size_of(ty) as i64;
                    size_bytes = Some(s * 2);
                }
                _ => {}
            }

            // default if unknown
            let length = AbstractExpression::Immediate(size_bytes.unwrap_or(4096));

            // insert memory region and add an abstract region mapping
            engine
                .computer
                .add_memory_region(name.clone(), RegionType::RW, length.clone());

            // set register abstract for this argument if we have a register mapping
            if arg_index < arg_regs.len() {
                let reg = arg_regs[arg_index];
                engine.computer.set_register_abstract(
                    reg,
                    Some(AbstractExpression::Abstract(name)),
                    0,
                );
            }
            arg_index += 1;
        }
    }

    let label = vars.item_fn.ident.to_string();
    let res = engine.start(&label);
    match res {
        Ok(_) => return token_stream,
        Err(err) => {
            #[cfg(not(debug_assertions))]
            emit_call_site_error!(err);
            #[cfg(debug_assertions)]
            emit_call_site_warning!(err);
            return token_stream;
        }
    }
}

// Minimal key=value parser for the memsafe_multiversion attribute.
struct VersionSpec {
    pub file: syn::LitStr,
    pub symbol: Option<syn::LitStr>,
    pub features: Vec<syn::LitStr>,
}

struct MultiAttr {
    pub versions: Vec<VersionSpec>,
    pub entry: Option<syn::LitStr>,
    pub fallback: Option<syn::Ident>,
    pub invariants: Vec<syn::Expr>,
    pub abi_probe: bool,
    pub abi_probe_sample_size: Option<usize>,
}

impl Parse for MultiAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut versions: Vec<VersionSpec> = Vec::new();
        let mut entry: Option<syn::LitStr> = None;
        let mut fallback: Option<syn::Ident> = None;
        let mut invariants: Vec<syn::Expr> = Vec::new();
        let mut abi_probe = false;
        let mut abi_probe_sample_size: Option<usize> = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let k = key.to_string();
            if k == "versions" {
                // parse versions as an array of tuples: [("file.s", "symbol", ["feat1"]), ...]
                let arr: syn::ExprArray = input.parse()?;
                for elem in arr.elems.iter() {
                    if let syn::Expr::Tuple(t) = elem {
                        if t.elems.len() < 1 {
                            return Err(input.error("version tuple must contain at least file string"));
                        }
                        // file
                        let file = match &t.elems[0] {
                            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => s.clone(),
                            _ => return Err(input.error("version file must be a string literal")),
                        };
                        // symbol (optional)
                        let symbol = if t.elems.len() > 1 {
                            match &t.elems[1] {
                                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => Some(s.clone()),
                                syn::Expr::Lit(_) => return Err(input.error("symbol must be a string literal")),
                                _ => None,
                            }
                        } else { None };
                        // features (optional)
                        let mut features: Vec<syn::LitStr> = Vec::new();
                        if t.elems.len() > 2 {
                            if let syn::Expr::Array(arr2) = &t.elems[2] {
                                for fe in &arr2.elems {
                                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = fe {
                                        features.push(s.clone());
                                    } else {
                                        return Err(input.error("features array must contain string literals"));
                                    }
                                }
                            } else {
                                return Err(input.error("features must be an array of string literals"));
                            }
                        }
                        versions.push(VersionSpec { file, symbol, features });
                    } else {
                        return Err(input.error("each version entry must be a tuple: (\"file\", \"symbol\", [\"feat\"])"));
                    }
                }
            } else if k == "entry" {
                entry = Some(input.parse::<syn::LitStr>()?);
            } else if k == "fallback" {
                // accept an identifier for fallback
                if input.peek(syn::Ident) {
                    fallback = Some(input.parse::<syn::Ident>()?);
                } else if input.peek(LitStr) {
                    let s = input.parse::<LitStr>()?;
                    fallback = Some(Ident::new(&s.value(), proc_macro2::Span::call_site()));
                } else {
                    return Err(input.error("fallback must be an identifier or string"));
                }
            } else if k == "invariants" {
                // parse an array of expressions
                let arr: syn::ExprArray = input.parse()?;
                for e in arr.elems.iter() {
                    invariants.push(e.clone());
                }
            } else if k == "abi_probe" {
                // expect boolean literal
                let ex: syn::Expr = input.parse()?;
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Bool(b), .. }) = ex {
                    abi_probe = b.value;
                } else {
                    return Err(input.error("abi_probe must be a boolean literal"));
                }
            } else if k == "abi_probe_sample_size" {
                let lit: syn::LitInt = input.parse()?;
                let v = lit.base10_parse::<usize>()?;
                abi_probe_sample_size = Some(v);
            } else {
                return Err(input.error(format!("unknown key '{}'", k)));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(MultiAttr {
            versions,
            entry,
            fallback,
            invariants,
            abi_probe,
            abi_probe_sample_size,
        })
    }
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn memsafe_multiversion(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse attribute and function item
    let attr = parse_macro_input!(attr as MultiAttr);
    let input_fn = parse_macro_input!(item as ItemFn);

    // extract function info
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let fn_ident = sig.ident.clone();
    let fn_name_string = fn_ident.to_string();

    // decide entry label inside asm
    let entry_label = if let Some(e) = &attr.entry { e.value() } else { fn_name_string.clone() };

    // For each version, run memsafe-checker to prove its assembly safe under the harness
    let cfg = Config::new();
    let ctx = Context::new(&cfg);

    struct VariantInfo {
        symbol_str: String,
        rust_ident: syn::Ident,
        features: Vec<String>,
        filename: String,
    }
    let mut variants: Vec<VariantInfo> = Vec::new();

    for (vi, vspec) in attr.versions.iter().enumerate() {
        let filename = vspec.file.value();
        let assembly_file: std::path::PathBuf = [std::env::var("OUT_DIR").expect("OUT_DIR"), filename.clone()].iter().collect();
        let f = File::open(&assembly_file).unwrap_or_else(|e| abort_call_site!(e));
        let reader = BufReader::new(f);
        let mut program: Vec<String> = Vec::new();
        for line in reader.lines() { program.push(line.unwrap_or(String::from(""))); }

        let mut engine = bums::x86_64::ExecutionEngineX86::from_asm_file(assembly_file.to_str().unwrap(), &ctx);

        // Map function inputs to memory regions & register abstracts
        let arg_regs = vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
        let mut arg_index = 0usize;
        for input in &sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                let name = match &*pat_type.pat { Pat::Ident(id) => id.ident.to_string(), _ => continue };
                let mut size_bytes: Option<i64> = None;
                match &*pat_type.ty {
                    Type::Array(a) => { let s = calculate_size_of_array(a) as i64; size_bytes = Some(s); }
                    Type::Reference(a) => match &*a.elem {
                        Type::Array(b) => { let s = calculate_size_of_array(b) as i64; size_bytes = Some(s); }
                        Type::Path(p) => { let ty = p.path.segments[0].ident.to_string(); let s = calculate_size_of(ty) as i64; size_bytes = Some(s); }
                        _ => {}
                    },
                    Type::Path(p) => { let ty = p.path.segments[0].ident.to_string(); let s = calculate_size_of(ty) as i64; size_bytes = Some(s * 2); }
                    _ => {}
                }
                let length = AbstractExpression::Immediate(size_bytes.unwrap_or(4096));
                engine.computer.add_memory_region(name.clone(), RegionType::RW, length.clone());
                if arg_index < arg_regs.len() {
                    let reg = arg_regs[arg_index];
                    engine.computer.set_register_abstract(reg, Some(AbstractExpression::Abstract(name)), 0);
                }
                arg_index += 1;
            }
        }

        // apply invariants
        for inv in &attr.invariants {
            if let syn::Expr::Binary(b) = inv {
                let ac = binary_to_abstract_comparison(b);
                let c = comparison_to_ast(engine.computer.context, ac).expect("engine6.5").simplify();
                engine.computer.solver.assert(&c);
            } else {
                abort_call_site!("invariants must be binary expressions");
            }
        }

        // run engine from entry label
        let res = engine.start(&entry_label);
        if let Err(err) = res {
            // Build enhanced diagnostic information to help the user
            let mut diag = String::new();
            diag.push_str(&format!("variant {} proof failure:\n{}\n", filename, err));

            // include a short assembly listing (with line numbers) to provide context
            diag.push_str("-- asm (first 200 lines) --\n");
            for (i, l) in program.iter().enumerate().take(200) {
                diag.push_str(&format!("{:5}: {}\n", i + 1, l));
            }

            // include any memory labels and relocations recorded by the engine
            if let Ok(rels) = std::panic::catch_unwind(|| engine.list_relocations()) {
                let rels: Vec<String> = rels;
                if !rels.is_empty() {
                    diag.push_str("-- relocations --\n");
                    for r in rels.iter() {
                        diag.push_str(&format!("{}\n", r));
                    }
                }
            }
            if let Ok(labels) = std::panic::catch_unwind(|| engine.dump_memory_labels()) {
                let labels: Vec<String> = labels;
                if !labels.is_empty() {
                    diag.push_str("-- memory labels --\n");
                    for l in labels.iter() {
                        diag.push_str(&format!("{}\n", l));
                    }
                }
            }

            #[cfg(not(debug_assertions))]
            emit_call_site_error!(diag);
            #[cfg(debug_assertions)]
            emit_call_site_warning!(diag);
            return TokenStream::new();
        }

        // prepare variant info
        let symbol_str = if let Some(s) = &vspec.symbol { s.value() } else { format!("{}_v{}", fn_name_string, vi) };
        // create a unique Rust identifier for the extern declaration and use link_name to bind
        // Include the wrapper function name and variant index to avoid collisions across the crate.
        let rust_name = format!("__asm_{}_v{}", fn_name_string, vi);
        let rust_ident = Ident::new(&rust_name, proc_macro2::Span::call_site());
        let features: Vec<String> = vspec.features.iter().map(|s| s.value()).collect();
        variants.push(VariantInfo { symbol_str, rust_ident, features, filename });
    }

    // fallback must be provided (for this MVP we require a Rust fallback)
    let fallback_ident = if let Some(id) = attr.fallback.clone() {
        id
    } else {
        abort_call_site!("memsafe_multiversion: missing fallback identifier (e.g. fallback = my_rust_impl)");
    };

    // collect argument idents to forward calls
    let mut arg_names: Vec<proc_macro2::TokenStream> = Vec::new();
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            match &*pat_type.pat {
                Pat::Ident(id) => {
                    let nm = id.ident.clone();
                    arg_names.push(quote! { #nm });
                }
                _ => {
                    abort_call_site!("unsupported argument pattern in function signature");
                }
            }
        }
    }

    // generate assert! statements from invariants expressions (use original syn::Expr tokens)
    let mut assert_tokens = proc_macro2::TokenStream::new();
    for inv in &attr.invariants {
        let e = inv.clone();
        assert_tokens.extend(quote! { assert!(#e); });
    }

    // Names for statics and probe function to avoid collisions
    let once_ident = Ident::new(&format!("__{}_memsafe_once", fn_name_string), proc_macro2::Span::call_site());
    let sel_ident = Ident::new(&format!("__{}_memsafe_sel", fn_name_string), proc_macro2::Span::call_site());
    let probe_ident = Ident::new(&format!("__{}_memsafe_abi_probe", fn_name_string), proc_macro2::Span::call_site());

    // ABI probe helper: best-effort attempt to detect callee-saved register clobber.
    // This uses inline asm and is best-effort; segfaults cannot be caught.
    let abi_probe_sample_size = attr.abi_probe_sample_size.unwrap_or(4096usize);
    let abi_probe_enabled = attr.abi_probe;

    // Derive a conservative sample size from invariants (e.g., `buf_len >= N`). If any
    // invariant requires a larger buffer than the provided sample size, use the larger.
    let mut probe_sample = abi_probe_sample_size;

    // helper: try to evaluate an expression to a usize if it contains only integer
    // literals and integer binary ops (+,-,*,/)
    fn eval_usize_expr(e: &Expr) -> Option<usize> {
        match e {
            Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) => i.base10_parse::<usize>().ok(),
            Expr::Unary(u) => match &u.op {
                UnOp::Neg(_) => None,
                UnOp::Not(_) => None,
                _ => None,
            },
            Expr::Binary(b) => {
                let l = eval_usize_expr(&b.left)?;
                let r = eval_usize_expr(&b.right)?;
                match b.op {
                    BinOp::Add(_) => Some(l.wrapping_add(r)),
                    BinOp::Sub(_) => l.checked_sub(r),
                    BinOp::Mul(_) => Some(l.wrapping_mul(r)),
                    BinOp::Div(_) => if r == 0 { None } else { Some(l / r) },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    for inv in &attr.invariants {
        if let syn::Expr::Binary(b) = inv {
            match b.op {
                BinOp::Ge(_) | BinOp::Gt(_) => {
                    // prefer a right-hand constant if available
                    if let Some(v) = eval_usize_expr(&b.right) { if v > probe_sample { probe_sample = v; } }
                    else if let Some(v) = eval_usize_expr(&b.left) { if v > probe_sample { probe_sample = v; } }
                }
                _ => {}
            }
        }
    }
    let abi_probe_sample_size_ts = proc_macro2::Literal::usize_unsuffixed(probe_sample);

    // Build an ABI probe that accepts a candidate function pointer matching the
    // user's signature and constructs simple sample arguments:
    // - raw pointer params receive `sample.as_mut_ptr() as <ptr-type>`
    // - `usize` params receive `sample.len()`
    // - integer params receive `0 as <int-type>`
    // This is intentionally conservative and best-effort.
    // Precompute parameter types and simple sample call arguments for the probe and
    // for creating typed function-pointer bindings below.
    let mut probe_param_tys: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut probe_call_args: Vec<proc_macro2::TokenStream> = Vec::new();
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            let ty = &*pat_type.ty;
            probe_param_tys.push(quote! { #ty });
            let arg_expr = match ty {
                Type::Ptr(_) => quote! { sample.as_mut_ptr() as #ty },
                Type::Reference(_) => quote! { sample.as_mut_ptr() as *mut _ as #ty },
                Type::Path(p) => {
                    let ident = p.path.segments.last().unwrap().ident.to_string();
                    match ident.as_str() {
                        "usize" => quote! { sample.len() },
                        "isize" => quote! { sample.len() as isize },
                        "u32" | "u64" | "u128" | "u16" | "u8" | "i32" | "i64" | "i128" | "i16" | "i8" => {
                            quote! { 0 as #ty }
                        }
                        _ => quote! { 0 as #ty },
                    }
                }
                _ => quote! { 0 as #ty },
            };
            probe_call_args.push(arg_expr);
        }
    }

    let abi_probe_fn = if abi_probe_enabled {
        // return type token stream for the probe (ensure explicit `-> T` form)
        let probe_ret_ts: proc_macro2::TokenStream = match &sig.output {
            ReturnType::Default => quote! { -> () },
            ReturnType::Type(_, ty) => quote! { -> #ty },
        };

        quote! {
            #[inline]
            unsafe fn #probe_ident(candidate: unsafe extern "C" fn( #(#probe_param_tys),* ) #probe_ret_ts, sample_size: usize) -> bool {
                use std::io::{Read, Write};

                // create a pipe for parent-child communication
                let mut fds: [i32;2] = [0,0];
                extern "C" {
                    fn pipe(fds: *mut i32) -> i32;
                    fn fork() -> i32;
                    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
                    fn _exit(code: i32) -> !;
                    fn close(fd: i32) -> i32;
                }

                if pipe(fds.as_mut_ptr()) != 0 { return false; }

                let pid = fork();
                if pid == 0 {
                    // child
                    // close read end
                    let _ = close(fds[0]);
                    let mut sample = vec![0u8; sample_size];

                    // capture regs before
                    let (rbx_b, r12_b, r13_b, r14_b, r15_b, rsp_b): (u64,u64,u64,u64,u64,u64);
                    core::arch::asm!(
                        "mov {0}, rbx\n\tmov {1}, r12\n\tmov {2}, r13\n\tmov {3}, r14\n\tmov {4}, r15\n\tmov {5}, rsp",
                        out(reg) rbx_b, out(reg) r12_b, out(reg) r13_b, out(reg) r14_b, out(reg) r15_b, out(reg) rsp_b
                    );

                    // write to pipe (use explicit type to avoid inference issues)
                    let mut w: std::fs::File = unsafe { std::os::unix::io::FromRawFd::from_raw_fd(fds[1]) };
                    let _ = w.write_all(&rbx_b.to_ne_bytes());
                    let _ = w.write_all(&r12_b.to_ne_bytes());
                    let _ = w.write_all(&r13_b.to_ne_bytes());
                    let _ = w.write_all(&r14_b.to_ne_bytes());
                    let _ = w.write_all(&r15_b.to_ne_bytes());
                    let _ = w.write_all(&rsp_b.to_ne_bytes());

                    // call candidate
                    let _ = candidate( #(#probe_call_args),* );

                    // capture regs after
                    let (rbx_a, r12_a, r13_a, r14_a, r15_a, rsp_a): (u64,u64,u64,u64,u64,u64);
                    core::arch::asm!(
                        "mov {0}, rbx\n\tmov {1}, r12\n\tmov {2}, r13\n\tmov {3}, r14\n\tmov {4}, r15\n\tmov {5}, rsp",
                        out(reg) rbx_a, out(reg) r12_a, out(reg) r13_a, out(reg) r14_a, out(reg) r15_a, out(reg) rsp_a
                    );

                    let _ = w.write_all(&rbx_a.to_ne_bytes());
                    let _ = w.write_all(&r12_a.to_ne_bytes());
                    let _ = w.write_all(&r13_a.to_ne_bytes());
                    let _ = w.write_all(&r14_a.to_ne_bytes());
                    let _ = w.write_all(&r15_a.to_ne_bytes());
                    let _ = w.write_all(&rsp_a.to_ne_bytes());
                    // ensure data is flushed and fd closed
                    drop(w);
                    unsafe { _exit(0); }
                } else if pid > 0 {
                    // parent
                    // close write end
                    let _ = close(fds[1]);
                    // wait for child
                    let mut status: i32 = 0;
                    let _ = waitpid(pid, &mut status as *mut i32, 0);
                    // if child terminated due to signal -> failure
                    if (status & 0x7f) != 0 { let _ = close(fds[0]); return false; }

                    // read before/after registers (6 u64 before, 6 u64 after)
                    let mut r: std::fs::File = unsafe { std::os::unix::io::FromRawFd::from_raw_fd(fds[0]) };
                    let mut buf_before = [0u8; 8*6];
                    let mut buf_after = [0u8; 8*6];
                    use std::io::Read;
                    if let Err(_) = r.read_exact(&mut buf_before) { return false; }
                    if let Err(_) = r.read_exact(&mut buf_after) { return false; }

                    let mut u64_from = |b: &[u8]| -> [u64;6] {
                        let mut out = [0u64;6];
                        for i in 0..6 { let mut arr = [0u8;8]; arr.copy_from_slice(&b[i*8..i*8+8]); out[i] = u64::from_ne_bytes(arr); }
                        out
                    };
                    let before = u64_from(&buf_before);
                    let after = u64_from(&buf_after);
                    // compare registers: all must be preserved
                    let preserved = before[0]==after[0] && before[1]==after[1] && before[2]==after[2] && before[3]==after[3] && before[4]==after[4] && before[5]==after[5];
                    let _ = close(fds[0]);
                    return preserved;
                } else {
                    // fork failed
                    let _ = close(fds[0]);
                    let _ = close(fds[1]);
                    return false;
                }
            }
        }
    } else {
        quote! {}
    };

    // (no typed candidate cast helper here)

    // build.rs snippet for user to copy/paste
    // Build.rs snippet will be emitted after proving variants (below)

    // generate the wrapper function; selection logic implemented per-variant below

    // Build extern declarations for each variant and selection logic
    let mut extern_blocks = proc_macro2::TokenStream::new();
    // We'll also prepare typed local bindings inside the wrapper that cast the
    // extern items to concrete function-pointer types derived from the signature.
    let mut typed_bindings_ts = proc_macro2::TokenStream::new();
    // Build the function-pointer type tokenstream for the wrapper signature
    let mut param_tys_for_ptr: Vec<proc_macro2::TokenStream> = Vec::new();
    for input in &sig.inputs {
        if let FnArg::Typed(pt) = input { let ty = &*pt.ty; param_tys_for_ptr.push(quote! { #ty }); }
    }
    // Normalize return type: ensure we always produce an explicit `-> Type` tokenstream
    // for building function-pointer types. If the function returns nothing, use `()`.
    let ret_ts: proc_macro2::TokenStream = match &sig.output {
        ReturnType::Default => quote! { -> () },
        ReturnType::Type(_, ty) => {
            quote! { -> #ty }
        }
    };

    // Build a list of parameter declarations (patterns with types) for extern decls
    let mut param_decls: Vec<proc_macro2::TokenStream> = Vec::new();
    for input in &sig.inputs {
        if let FnArg::Typed(pt) = input {
            param_decls.push(quote! { #pt });
        }
    }

    for v in &variants {
        let extern_ident = v.rust_ident.clone();
        let sym_lit = syn::LitStr::new(&v.symbol_str, proc_macro2::Span::call_site());
        // Emit extern declaration with explicit return type tokenstream (ret_ts)
        extern_blocks.extend(quote! {
            extern "C" { #[link_name = #sym_lit] fn #extern_ident( #(#param_decls),* ) #ret_ts; }
        });

        // typed binding name
        let bind_ident = Ident::new(&format!("__asm_ptr_{}", v.rust_ident), proc_macro2::Span::call_site());
        // the extern item is named v.rust_ident; create a static fn-pointer binding with explicit type
        let extern_ident = v.rust_ident.clone();
        typed_bindings_ts.extend(quote! {
            static #bind_ident: unsafe extern "C" fn( #(#param_tys_for_ptr),* ) #ret_ts = #extern_ident as unsafe extern "C" fn( #(#param_tys_for_ptr),* ) #ret_ts;
        });
    }

    // Prepare ABI probe boolean literal for generated code
    let abi_probe_lit_ts = if abi_probe_enabled { quote! { true } } else { quote! { false } };

    // Build feature checks and selection arms
    let mut selection_arms = proc_macro2::TokenStream::new();
    for (idx, v) in variants.iter().enumerate() {
        let _ident = &v.rust_ident;
        let bind_ident_name = format!("__asm_ptr_{}", v.rust_ident);
        let bind_ident = Ident::new(&bind_ident_name, proc_macro2::Span::call_site());
        let idx_lit = idx as u8;
        if v.features.is_empty() {
            // always-available candidate
            if abi_probe_enabled {
                selection_arms.extend(quote! {
                    // candidate without feature requirement; only choose if none chosen yet
                    if chosen == 255u8 {
                        if unsafe { #probe_ident(#bind_ident, #abi_probe_sample_size_ts) } {
                            chosen = #idx_lit;
                        }
                    }
                });
            } else {
                // choose only if none chosen yet (preserve user order -> first wins)
                selection_arms.extend(quote! { if chosen == 255u8 { chosen = #idx_lit; } });
            }
        } else {
            // build combined feature check
            let mut feats_ts = proc_macro2::TokenStream::new();
            for f in &v.features {
                let lit = syn::LitStr::new(&f, proc_macro2::Span::call_site());
                feats_ts.extend(quote! { std::is_x86_feature_detected!(#lit) && });
            }
            // remove trailing && by wrapping in a block
            selection_arms.extend(quote! {
                if { let cond = { #feats_ts true }; cond } {
                    if chosen == 255u8 {
                        if #abi_probe_lit_ts {
                            if unsafe { #probe_ident(#bind_ident, #abi_probe_sample_size_ts) } { chosen = #idx_lit; }
                        } else {
                            chosen = #idx_lit;
                        }
                    }
                }
            });
        }
    }

    // build environment override detection code
    let mut env_override_ts = proc_macro2::TokenStream::new();
    // allow overriding by symbol name or feature name
    let mut override_arms = proc_macro2::TokenStream::new();
    for (idx, v) in variants.iter().enumerate() {
        let idx_lit = idx as u8;
        let sym = &v.symbol_str;
        let sym_lit = syn::LitStr::new(sym, proc_macro2::Span::call_site());
        override_arms.extend(quote! {
            if val == #sym_lit { chosen = #idx_lit; }
        });
        for feat in &v.features {
            let feat_lit = syn::LitStr::new(feat, proc_macro2::Span::call_site());
            override_arms.extend(quote! {
                if val == #feat_lit { chosen = #idx_lit; }
            });
        }
    }
    env_override_ts.extend(quote! {
        if let Ok(val) = std::env::var("BUMS_FORCE_IMPL") {
            #override_arms
        }
    });

    // Emit a build.rs snippet to help the user copy assembly files into OUT_DIR and compile them with cc
    // Collect unique filenames
    let mut uniq_files = std::collections::HashSet::new();
    for v in &variants { uniq_files.insert(v.filename.clone()); }
    let mut snippet = String::new();
    snippet.push_str("// Paste this into your build.rs to copy asm files into OUT_DIR and assemble them with cc::Build\n");
    snippet.push_str("use std::path::PathBuf; use std::fs;\nfn main() {\n    let out = PathBuf::from(std::env::var(\"OUT_DIR\").unwrap());\n    fs::create_dir_all(&out).unwrap();\n");
    for f in uniq_files {
        let dst = f.clone();
        let src = format!("../../{}", f);
        let compile_name = dst.replace('.', "_");
        snippet.push_str(&format!("    let src = PathBuf::from(\"{}\");\n    let dst = out.join(\"{}\");\n    let _ = fs::copy(&src, &dst).expect(\"copy asm\");\n    cc::Build::new().file(dst).flag_if_supported(\"-masm=intel\").compile(\"{}\");\n\n", src, dst, compile_name));
    }
    snippet.push_str("}\n");
    emit_call_site_warning!(format!("memsafe_multiversion build.rs snippet:\n{}", snippet));

    // selection code executed once
    let selection_block = quote! {
        #once_ident.call_once(|| {
            let mut chosen: u8 = 255u8; // 255 = none
            // env override
            #env_override_ts
            if chosen == 255u8 {
                // try each candidate in user order
                #selection_arms
            }
            // if still none, fallback is represented by 255
            unsafe { #sel_ident = chosen; }
        });
    };

    // dispatch code: if sel < variants.len() call that variant else fallback
    // Use the typed local bindings created earlier (__asm_ptr_<ident>) to ensure
    // the call sites have fully concrete types and do not require inference.
    let mut dispatch_match = proc_macro2::TokenStream::new();
    for (idx, v) in variants.iter().enumerate() {
        let idx_lit = idx as u8;
        let bind_ident = Ident::new(&format!("__asm_ptr_{}", v.rust_ident), proc_macro2::Span::call_site());
        dispatch_match.extend(quote! {
            if sel == #idx_lit { unsafe { return #bind_ident( #(#arg_names),* ); } }
        });
    }

    let output = quote! {
        #extern_blocks

        #abi_probe_fn

        #typed_bindings_ts

        static #once_ident: std::sync::Once = std::sync::Once::new();
        static mut #sel_ident: u8 = 255u8;

        #vis #sig {
        #assert_tokens
            #selection_block
            let sel = unsafe { #sel_ident };
            #dispatch_match
            // fallback
            return #fallback_ident( #(#arg_names),* );
        }
    };

    // For debugging: try to write the generated output to OUT_DIR/<fn>_expanded.rs
    let _ = std::panic::catch_unwind(|| {
        if let Ok(outdir) = std::env::var("OUT_DIR") {
            let mut path = std::path::PathBuf::from(outdir);
            path.push(format!("{}_memsafe_expanded.rs", fn_name_string));
            let _ = std::fs::write(path, output.to_string());
        }
        // Also write a copy to /tmp to aid debugging in test runs
        let _ = std::panic::catch_unwind(|| {
            let _ = std::fs::write(format!("/tmp/{}_memsafe_expanded.rs", fn_name_string), output.to_string());
        });
    });
    output.into()
}
