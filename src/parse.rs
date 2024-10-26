use chumsky::{extra::Err, prelude::*};

// The following `parser()` function, aside from some tweaks for personal use
// case, is derived primarily from Chumsky's `foo` example. Chumsky's repository
// is distributed with the following license.
//
// The MIT License (MIT)
//
// Copyright (c) 2021 Joshua Barretto
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

type Spanned<T> = (T, SimpleSpan);

pub fn parser<'src>() -> impl Parser<'src, &'src str, Expr, Err<Rich<'src, char>>> {
    let ident = text::ascii::ident()
        .padded()
        .map_with(|ident: &str, extra| (ident.to_owned(), extra.span()));

    let expr = recursive(|expr| {
        let int = text::int(10).map_with(|s: &str, extra|
            Expr::Num(s.parse().unwrap(), Some(extra.span()))
        );

        let call =
            ident
            .then(
                expr.clone()
                    .separated_by(just(','))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just('('), just(')')),
            )
            .map_with(|(f, args), extra|
                Expr::Call(f, args, Some(extra.span()))
            );

        let atom =
            int
            .or(expr.delimited_by(just('('), just(')')))
            .or(call)
            .or(
                ident.map(|(ident, span)| Expr::Var(ident, Some(span)))
            )
            .padded();

        let op = |c| just(c).padded();

        let unary = op('-')
            .repeated() // <- allow any number of consecutive negative signs
            .foldr(atom, |_op, rhs| Expr::Neg(Box::new(rhs), None))
            .map_with(|mut expr, extra| { expr.set_span(extra.span()); expr });

        let product = unary.clone().foldl(
            choice(( // tuple structs are implicitly functions
                op('*').to(Expr::Mul as fn(_, _, _) -> _),
                op('/').to(Expr::Div as fn(_, _, _) -> _),
            ))
            .then(unary)
            .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs), None),
        )
            .map_with(|mut expr, extra| { expr.set_span(extra.span()); expr });

        let sum = product.clone().foldl(
            choice((
                op('+').to(Expr::Add as fn(_, _, _) -> _),
                op('-').to(Expr::Sub as fn(_, _, _) -> _),
            ))
            .then(product)
            .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs), None),
        )
            .map_with(|mut expr, extra| { expr.set_span(extra.span()); expr });

        sum
    });

    let decl = recursive(|decl| {
        let r#let = text::ascii::keyword("let")
            .ignore_then(ident)
            .then_ignore(just('='))
            .then(expr.clone())
            .then_ignore(just(';'))
            .then(decl.clone())
            .map_with(|((name, rhs), then), extra | Expr::Let {
                name,
                rhs: Box::new(rhs),
                then: Box::new(then),
                span: Some(extra.span()),
            });

        let r#fn = text::ascii::keyword("fn")
            .ignore_then(ident)
            .then(
                ident.repeated()
                    .collect::<Vec<_>>()
            )
            .then_ignore(just('='))
            .then(expr.clone())
            .then_ignore(just(';'))
            .then(decl)
            .map_with(|(((name, args), body), then), extra|
                Expr::Fn {
                    name,
                    args,
                    body: Box::new(body),
                    then: Box::new(then),
                    span: Some(extra.span()),
                }
            );

        r#let.or(r#fn).or(expr).padded()
    });

    decl
}

/// Abstract Syntax Tree for Foo. This is modified from Chumsky's example to
/// include spans for diagnostic reporting.
#[derive(Debug)]
pub enum Expr {
    Num(f64, Option<SimpleSpan>),
    Var(String, Option<SimpleSpan>),

    Neg(Box<Expr>, Option<SimpleSpan>),
    Add(Box<Expr>, Box<Expr>, Option<SimpleSpan>),
    Sub(Box<Expr>, Box<Expr>, Option<SimpleSpan>),
    Mul(Box<Expr>, Box<Expr>, Option<SimpleSpan>),
    Div(Box<Expr>, Box<Expr>, Option<SimpleSpan>),

    Call(Spanned<String>, Vec<Expr>, Option<SimpleSpan>),
    Let {
        name: Spanned<String>,
        rhs: Box<Expr>,
        then: Box<Expr>,
        span: Option<SimpleSpan>,
    },
    Fn {
        name: Spanned<String>,
        args: Vec<Spanned<String>>,
        body: Box<Expr>,
        then: Box<Expr>,
        span: Option<SimpleSpan>,
    },
}

impl Expr {
    /// Fill the `span` field of any of the `Expr` types, regardless of type.
    /// Some of the parsers construct the `Expr` before calling `map_with()` to
    /// add the span, so this method saves on in-parser logic.
    pub fn set_span(&mut self, span: SimpleSpan) {
        let s = match self {
            Expr::Num(_, s) => s,
            Expr::Var(_, s) => s,
            Expr::Neg(_, s) => s,
            Expr::Add(_, _, s) => s,
            Expr::Sub(_, _, s) => s,
            Expr::Mul(_, _, s) => s,
            Expr::Div(_, _, s) => s,
            Expr::Call(_, _, s) => s,
            Expr::Let { span: s, .. } => s,
            Expr::Fn { span: s, .. } => s,
        };
        *s = Some(span);
    }
}