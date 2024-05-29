//! Provides implementation of the important traits from the [visitor] module for some types.
//! These types are:
//! - [`Infallible`]:
//!
//!   For when you want to indicate that the [`Some`]/[`ControlFlow::Continue`][std::ops::ControlFlow::Continue] value is never returned.
//!
//!   This is needed, as some methods return `Option<T>` or `ControlFlow<Self, (A, B)>`, and a value of [`None`]
//!   or [`ControlFlow::Break`][std::ops::ControlFlow::Break] indicates no interest in the item.
//!
//! - The Unit Type `()`:
//!
//!   For when you also want to void something, but actually invoke all of the visitor methods.
//!
//!   This could for example be used to test the class reading.
//!
//! - The appropriate storage structure from this crate. This includes the following:
//!   - [`Vec`]<[`ClassFile`]> implements [`MultiClassVisitor`]
//!   - [`ClassFile`] implements [`ClassVisitor`]
//!
//! [visitor]: crate::visitor
//! [`Infallible`]: std::convert::Infallible
//! [`ClassFile`]: crate::tree::class::ClassFile
//! [`MultiClassVisitor`]: crate::visitor::MultiClassVisitor
//! [`ClassVisitor`]: crate::visitor::class::ClassVisitor

// TODO: document more implementations

mod tree;
mod unit_tuple;
mod infallible;