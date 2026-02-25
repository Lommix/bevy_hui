use crate::{
    adaptor::AssetServerAdaptor,
    build::{
        ContentId, HtmlNode, Tags, TemplateExpresions, TemplateProperties,
        TemplatePropertySubscriber, TemplateScope,
    },
    data::HtmlTemplate,
    styles::HtmlStyle,
};
use bevy::prelude::*;
use nom::{
    bytes::complete::{is_not, tag, take_until},
    character::complete::multispace0,
    sequence::{delimited, preceded, tuple},
};

pub struct CompilePlugin;
impl Plugin for CompilePlugin {
    fn build(&self, app: &mut App) {
        // app.add_event::<CompileNodeEvent>();
        // app.add_event::<CompileContextEvent>();
        // app.add_event::<CompileContentEvent>();
        app.add_observer(compile_node);
        app.add_observer(compile_context);
        app.add_observer(compile_text);
    }
}

#[derive(Event)]
pub struct CompileContentEvent {
    pub entity: Entity,
}

fn compile_text(
    trigger: On<CompileContentEvent>,
    mut nodes: Query<(&ContentId, &TemplateScope, &mut Text)>,
    root: Query<(&HtmlNode, &TemplateProperties)>,
    templates: Res<Assets<HtmlTemplate>>,
) {
    let entity = trigger.entity;
    let Ok((content_id, scope, mut text)) = nodes.get_mut(entity) else {
        warn!("trying to compile content for {entity}, that does not have any");
        return;
    };

    let Some((template, props)) = root
        .get(**scope)
        .ok()
        .map(|(handle, props)| templates.get(&**handle).map(|d| (d, props)))
        .flatten()
    else {
        warn!("{entity} has no scope!");
        return;
    };

    _ = template
        .content
        .get(**content_id)
        .map(|raw| compile_content(raw.trim(), &props))
        .map(|compiled| **text = compiled);
}

#[derive(Event)]
pub struct CompileNodeEvent {
    pub entity: Entity,
}

fn compile_node(
    trigger: On<CompileNodeEvent>,
    mut cmd: Commands,
    mut nodes: Query<(&mut HtmlStyle, &TemplateScope)>,
    mut images: Query<&mut ImageNode>,
    mut tags: Query<&mut Tags>,
    expressions: Query<&TemplateExpresions>,
    contexts: Query<&TemplateProperties>,
    server: Res<AssetServer>,
) {
    let entity = trigger.entity;
    let Ok((mut node_style, scope)) = nodes.get_mut(entity) else {
        // unbuild nodes also complain
        // warn!("Trying to compile a non ui node");
        return;
    };

    // check owned properties aswell
    let Some(context) = contexts.get(entity).ok().or(contexts.get(**scope).ok()) else {
        warn!("Node has no context scope");
        return;
    };

    if let Ok(expressions) = expressions.get(entity) {
        let mut adapter = AssetServerAdaptor { server: &server };
        expressions
            .iter()
            .for_each(|expr| match expr.compile(context, &mut adapter) {
                Some(compiled) => {
                    match compiled {
                        crate::data::Attribute::Style(style_attr) => {
                            node_style.add_style_attr(style_attr, Some(&server))
                        }
                        crate::data::Attribute::Action(action) => {
                            action.self_insert(cmd.entity(entity))
                        }
                        crate::data::Attribute::Path(path) => {
                            _ = images.get_mut(entity).map(|mut img| {
                                img.image = server.load(path);
                            });
                        }
                        crate::data::Attribute::Tag(key, value) => match tags.get_mut(entity) {
                            Ok(mut tags) => {
                                tags.insert(key, value);
                            }
                            Err(_) => {
                                warn!("node has to tags")
                            }
                        },
                        rest => {
                            warn!("attribute of this kind cannot be dynamic `{:?}`", rest);
                        }
                    };
                }
                None => {
                    dbg!(context);
                    warn!("expression failed to compile `{:?}`", expr);
                    return;
                }
            });
    }
}

#[derive(Event)]
pub struct CompileContextEvent {
    pub entity: Entity,
}

fn compile_context(
    trigger: On<CompileContextEvent>,
    expressions: Query<(&TemplateExpresions, Option<&TemplateScope>)>,
    text_nodes: Query<(), With<ContentId>>,
    subscriber: Query<&TemplatePropertySubscriber>,
    mut properties: Query<&mut TemplateProperties>,
    mut cmd: Commands,
    server: Res<AssetServer>,
) {
    let entity = trigger.entity;
    if let Ok((expressions, scope)) = expressions.get(entity) {
        // ----------
        // problem: compiling props on template root nodes
        // always look up the tree, but sometimes the first node
        // has props owned by its own prop context.

        // compile
        if let Some(parent_context) = scope.map(|s| properties.get(**s).ok()).flatten() {
            let mut adapter = AssetServerAdaptor { server: &server };
            let mut compiled_defintions = vec![];
            for expr in expressions.iter() {
                // --------------------
                //compile from parent
                match expr.compile(parent_context, &mut adapter) {
                    Some(compiled) => match compiled {
                        crate::data::Attribute::PropertyDefinition(key, value) => {
                            compiled_defintions.push((key, value));
                        }
                        _ => {
                            // error!("cannot compile to unimplementd attribute `{:?}`", compiled);
                        }
                    },
                    None => {
                        // check owned props
                        if let Ok(owned_ctx) = properties.get(entity) {
                            match expr.compile(owned_ctx, &mut adapter) {
                                Some(crate::data::Attribute::PropertyDefinition(key, value)) => {
                                    compiled_defintions.push((key, value));
                                }
                                _ => {
                                    // error!("cannot compile to unimplementd attribute `{:?}`", expr);
                                }
                            }
                        } else {
                            error!("cannot compile: {:#?}", expr);
                        }
                    }
                }
                // --------------------
                // compile from self
            }
            _ = properties.get_mut(entity).map(|mut context| {
                context.extend(compiled_defintions.into_iter());
            });
        };
    };

    if let Ok(subs) = subscriber.get(entity) {
        for sub in subs.iter() {
            if *sub != entity && properties.get(*sub).is_ok() {
                cmd.trigger(CompileContextEvent { entity: *sub });
            } else {
                cmd.trigger(CompileNodeEvent { entity: *sub });
            }
            if text_nodes.get(*sub).is_ok() {
                cmd.trigger(CompileContentEvent { entity: *sub });
            }
        }
    }
}

/// Finds the first `{variable}` occurrence in `input`.
///
/// Returns `(prefix, variable_name, remainder)` where:
/// - `prefix`       – literal text before the opening `{`
/// - `variable_name`– trimmed key between `{` and `}`
/// - `remainder`    – text after the closing `}`
///
/// Returns `None` when there is no `{…}` pattern in `input`.
pub(crate) fn find_template_var(input: &str) -> Option<(&str, &str, &str)> {
    let result: Result<(&str, (&str, &str)), nom::Err<nom::error::Error<&str>>> = tuple((
        take_until("{"),
        delimited(tag("{"), preceded(multispace0, is_not("}")), tag("}")),
    ))(input);

    match result {
        Ok((remainder, (prefix, var))) => Some((prefix, var.trim_end(), remainder)),
        Err(_) => None,
    }
}

/// Replace every `{variable}` token in `input` with the matching value from
/// `defs`, leaving unresolved tokens as empty strings.
pub(crate) fn compile_content(input: &str, defs: &TemplateProperties) -> String {
    let mut compiled = String::new();
    let mut remaining = input;

    while let Some((prefix, key, rest)) = find_template_var(remaining) {
        compiled.push_str(prefix);
        if let Some(value) = defs.get(key) {
            compiled.push_str(value);
        }
        remaining = rest;
    }

    // append any trailing literal text after the last `}`
    compiled.push_str(remaining);
    compiled
}
