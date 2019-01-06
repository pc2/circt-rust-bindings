// Copyright (c) 2016-2019 Fabian Schuiki

//! A parameter environment generated by an instantiation.

use crate::{
    ast_map::AstNode,
    crate_prelude::*,
    hir::{HirNode, NamedParam, PosParam},
};

/// A parameter environment.
///
/// This is merely an handle that is cheap to copy and pass around. Use the
/// [`Context`] to resolve this to the actual [`ParamEnvData`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamEnv(pub(crate) u32);

/// A parameter environment.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ParamEnvData {
    values: Vec<(NodeId, NodeId)>,
    types: Vec<(NodeId, NodeId)>,
}

/// A location that implies a parameter environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamEnvSource<'hir> {
    ModuleInst {
        module: NodeId,
        pos: &'hir [PosParam],
        named: &'hir [NamedParam],
    },
}

pub(crate) fn compute<'gcx>(
    cx: &impl Context<'gcx>,
    src: ParamEnvSource<'gcx>,
) -> Result<ParamEnv> {
    match src {
        ParamEnvSource::ModuleInst { module, pos, named } => {
            let module = match cx.hir_of(module)? {
                HirNode::Module(m) => m,
                _ => panic!("expected module"),
            };

            // Associate the positional and named assignments with the actual
            // parameters of the module.
            let param_iter = pos
                .iter()
                .enumerate()
                .map(
                    |(index, &(span, assign_id))| match module.params.get(index) {
                        Some(&param_id) => Ok((param_id, assign_id)),
                        None => {
                            cx.emit(
                                DiagBuilder2::error(format!(
                                    "{} only has {} parameter(s)",
                                    module.desc_full(),
                                    module.params.len()
                                ))
                                .span(span),
                            );
                            Err(())
                        }
                    },
                )
                .chain(named.iter().map(|&(_span, name, assign_id)| {
                    let names: Vec<_> = module
                        .params
                        .iter()
                        .flat_map(|&id| match cx.ast_of(id) {
                            Ok(AstNode::TypeParam(_, p)) => Some((p.name.name, id)),
                            Ok(AstNode::ValueParam(_, p)) => Some((p.name.name, id)),
                            Ok(_) => unreachable!(),
                            Err(()) => None,
                        })
                        .collect();
                    match names
                        .iter()
                        .find(|&(param_name, _)| *param_name == name.value)
                    {
                        Some(&(_, param_id)) => Ok((param_id, assign_id)),
                        None => {
                            cx.emit(
                                DiagBuilder2::error(format!(
                                    "no parameter `{}` in {}",
                                    name,
                                    module.desc_full(),
                                ))
                                .span(name.span)
                                .add_note(format!(
                                    "declared parameters are {}",
                                    names
                                        .iter()
                                        .map(|&(n, _)| format!("`{}`", n))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )),
                            );
                            Err(())
                        }
                    }
                }));
            let param_iter = param_iter
                .collect::<Vec<_>>()
                .into_iter()
                .collect::<Result<Vec<_>>>()?
                .into_iter();

            // Split up type and value parameters.
            let mut types = vec![];
            let mut values = vec![];
            for (param_id, assign_id) in param_iter {
                match cx.ast_of(param_id)? {
                    AstNode::TypeParam(..) => types.push((param_id, assign_id)),
                    AstNode::ValueParam(..) => values.push((param_id, assign_id)),
                    _ => unreachable!(),
                }
            }

            Ok(cx.intern_param_env(ParamEnvData { types, values }))
        }
    }
}
