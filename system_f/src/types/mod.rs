use crate::terms::{Kind, Literal, Term};
use std::collections::VecDeque;
use util::span::Span;

mod visit;
use visit::{MutVisitor, Shift, Subst};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Type {
    Unit,
    Nat,
    Bool,
    Var(usize),
    Arrow(Box<Type>, Box<Type>),
    Universal(Box<Type>),
}

pub enum Binding {}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct Context {
    stack: VecDeque<Type>,
}

impl Context {
    pub fn push(&mut self, ty: Type) {
        self.stack.push_front(ty);
    }

    pub fn pop(&mut self) {
        self.stack
            .pop_front()
            .expect("Context::pop() with empty type stack");
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn find(&self, idx: usize) -> Option<&Type> {
        self.stack.get(idx)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct TypeError {
    pub span: Span,
    pub kind: TypeErrorKind,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum TypeErrorKind {
    ParameterMismatch(Box<Type>, Box<Type>, Span),
    NotArrow,
    NotUniversal,
    UnboundVariable,
}

impl Context {
    pub const fn error(term: &Term, kind: TypeErrorKind) -> Result<Type, TypeError> {
        Err(TypeError {
            span: term.span,
            kind,
        })
    }

    pub fn type_of(&mut self, term: &Term) -> Result<Type, TypeError> {
        match term.kind() {
            Kind::Lit(Literal::Unit) => Ok(Type::Unit),
            Kind::Lit(Literal::Bool(_)) => Ok(Type::Bool),
            Kind::Lit(Literal::Nat(_)) => Ok(Type::Nat),
            Kind::Var(idx) => self.find(*idx).cloned().ok_or_else(|| TypeError {
                span: term.span,
                kind: TypeErrorKind::UnboundVariable,
            }),
            Kind::Abs(ty, t2) => {
                self.push(*ty.clone());
                let mut ty2 = self.type_of(t2)?;
                self.pop();

                Shift::new(-1).visit(&mut ty2);

                Ok(Type::Arrow(ty.clone(), Box::new(ty2)))
            }
            Kind::App(t1, t2) => {
                let ty1 = self.type_of(t1)?;
                let ty2 = self.type_of(t2)?;
                dbg!(t1.span);
                match ty1 {
                    Type::Arrow(ty11, ty12) => {
                        if *ty11 == ty2 {
                            Ok(*ty12)
                        } else {
                            Context::error(
                                t1,
                                TypeErrorKind::ParameterMismatch(ty11, Box::new(ty2), t2.span),
                            )
                        }
                    }
                    _ => Context::error(term, TypeErrorKind::NotArrow),
                }
            }
            Kind::Fix(inner) => {
                let ty = self.type_of(inner)?;
                match ty {
                    Type::Arrow(ty1, ty2) => {
                        if ty1 == ty2 {
                            Ok(*ty1)
                        } else {
                            Context::error(
                                term,
                                TypeErrorKind::ParameterMismatch(ty1, ty2, inner.span),
                            )
                        }
                    }
                    _ => Context::error(term, TypeErrorKind::NotArrow),
                }
            }
            Kind::TyAbs(ty, term) => {
                self.push(*ty.clone());
                let ty2 = self.type_of(term)?;
                self.pop();
                Ok(Type::Universal(Box::new(ty2)))
            }
            Kind::TyApp(term, ty) => {
                let mut ty = ty.clone();
                let ty1 = self.type_of(term)?;
                match ty1 {
                    Type::Universal(mut ty12) => {
                        Shift::new(1).visit(&mut ty);
                        Subst::new(*ty).visit(&mut ty12);
                        Ok(*ty12)
                    }
                    _ => Context::error(term, TypeErrorKind::NotUniversal),
                }
            }
        }
    }
}