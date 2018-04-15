// Copyright (c) 2018 Fabian Schuiki

//! Facilities to manage declarations and resolve names.
//!
//! TODO: Replace `scope` with this module.

#![deny(missing_docs)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use common::{SessionContext, Verbosity};
use common::errors::*;
use common::score::Result;
use common::source::Spanned;

use hir;
use score::ResolvableName;

/// A definition.
#[derive(Copy, Clone, Debug)]
pub enum Def2<'t> {
    /// A package.
    Pkg(&'t hir::Slot<'t, hir::Package2<'t>>),
    /// A type declaration.
    Type(&'t hir::Slot<'t, hir::TypeDecl2>),
    /// An enumeration type variant.
    Enum(()),
}

/// A scope.
#[derive(Clone, Debug)]
pub struct ScopeData<'t> {
    /// The parent scope.
    pub parent: Option<&'t ScopeData<'t>>,

    /// The definitions made in this scope.
    pub defs: RefCell<HashMap<ResolvableName, Vec<Spanned<Def2<'t>>>>>,

    /// The definitions imported from other scopes.
    pub imported_defs: RefCell<HashMap<ResolvableName, Vec<Spanned<Def2<'t>>>>>,

    /// The explicitly imported scopes.
    pub imported_scopes: RefCell<HashSet<&'t ScopeData<'t>>>,
}

impl<'t> ScopeData<'t> {
    /// Create a new root scope.
    pub fn root() -> ScopeData<'t> {
        ScopeData {
            parent: None,
            defs: RefCell::new(HashMap::new()),
            imported_defs: RefCell::new(HashMap::new()),
            imported_scopes: RefCell::new(HashSet::new()),
        }
    }

    /// Create a new scope.
    pub fn new(parent: &'t ScopeData<'t>) -> ScopeData<'t> {
        ScopeData {
            parent: Some(parent),
            ..Self::root()
        }
    }

    /// Define a new name in the scope.
    pub fn define(
        &self,
        name: Spanned<ResolvableName>,
        def: Def2<'t>,
        ctx: &SessionContext,
    ) -> Result<()> {
        if ctx.has_verbosity(Verbosity::NAMES) {
            ctx.emit(
                DiagBuilder2::note(format!("define `{}` as {:?}", name.value, def)).span(name.span),
            );
        }
        debugln!("define `{}` as {:?}", name.value, def);
        match def {
            // Handle overloadable cases.
            Def2::Enum(..) => {
                self.defs
                    .borrow_mut()
                    .entry(name.value)
                    .or_insert_with(|| Vec::new())
                    .push(Spanned::new(def, name.span));
                Ok(())
            }

            // Handle unique cases.
            _ => {
                let ins = self.defs
                    .borrow_mut()
                    .insert(name.value, vec![Spanned::new(def, name.span)]);
                if let Some(existing) = ins {
                    ctx.emit(
                        DiagBuilder2::error(format!("`{}` has already been declared", name.value))
                            .span(name.span)
                            .add_note("Previous declaration was here:")
                            .span(existing.last().unwrap().span),
                    );
                    Err(())
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Import a definition into the scope.
    pub fn import_def(&self, name: Spanned<ResolvableName>, def: Def2<'t>) -> Result<()> {
        self.imported_defs
            .borrow_mut()
            .entry(name.value)
            .or_insert_with(|| Vec::new())
            .push(Spanned::new(def, name.span));
        Ok(())
    }

    /// Import an entire scope into the scope.
    pub fn import_scope(&self, scope: &'t ScopeData<'t>) -> Result<()> {
        self.imported_scopes.borrow_mut().insert(scope);
        Ok(())
    }
}

impl<'t> PartialEq for &'t ScopeData<'t> {
    fn eq(&self, b: &Self) -> bool {
        (*self) as *const _ == (*b) as *const _
    }
}
impl<'t> Eq for &'t ScopeData<'t> {}

impl<'t> Hash for &'t ScopeData<'t> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        Hash::hash(&((*self) as *const _), hasher)
    }
}

/// Define names and perform name resolution.
pub trait ScopeContext<'t> {
    /// Define a new name in the scope.
    fn define(&self, name: Spanned<ResolvableName>, def: Def2<'t>) -> Result<()>;
    /// Import a definition into the scope.
    fn import_def(&self, name: Spanned<ResolvableName>, def: Def2<'t>) -> Result<()>;
    /// Import an entire scope into the scope.
    fn import_scope(&self, scope: &'t ScopeData<'t>) -> Result<()>;
}
