use bevy::{prelude::*, ui::Pressed};
use bevy_hui::prelude::*;

/// # Select Widget
///
/// A select is a button with 2 children. The current
/// selected node and a hidden node, holding the options.
///
pub struct HuiSelectWidgetPlugin;
impl Plugin for HuiSelectWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SelectInput>();
        app.register_type::<SelectOption>();
        app.register_type::<SelectionChangedEvent>();
        app.add_message::<SelectionChangedEvent>();
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            (
                open_list,
                selection,
                update_selection.run_if(on_message::<SelectionChangedEvent>),
            ),
        );
    }
}

#[derive(Component, Default, Reflect, Debug)]
#[reflect]
pub struct SelectInput {
    // points to the current select node
    pub value: Option<Entity>,
}

#[derive(Component, Debug, Reflect)]
#[reflect]
pub struct SelectOption {
    select: Entity,
}

#[derive(Message, Reflect, Debug)]
#[reflect]
pub struct SelectionChangedEvent {
    pub select: Entity,
    pub option: Entity,
}

fn setup(mut html_funcs: HtmlFunctions) {
    html_funcs.register("init_select", init_select);
}

fn init_select(
    In(entity): In<Entity>,
    mut cmd: Commands,
    children: Query<&Children>,
    targets: Query<&UiTarget>,
) {
    cmd.entity(entity).insert(SelectInput::default());

    let Ok(option_holder) = targets.get(entity) else {
        warn!("your select does not have a target option list");
        return;
    };

    _ = children.get(**option_holder).map(|children| {
        children.iter().for_each(|option| {
            cmd.entity(option).insert(SelectOption { select: entity });
        });
    });
}

fn open_list(
    selects: Query<(&UiTarget, Has<Pressed>), (With<SelectInput>, Added<Pressed>)>,
    mut styles: Query<&mut HtmlStyle>,
) {
    for (target, pressed) in selects.iter() {
        if !pressed {
            continue;
        }

        let Ok(mut list_style) = styles.get_mut(**target) else {
            continue;
        };

        list_style.computed.node.display = Display::Grid;
    }
}

fn selection(
    mut events: MessageWriter<SelectionChangedEvent>,
    options: Query<(Entity, &ChildOf, Has<Pressed>, &SelectOption), Added<Pressed>>,
    mut styles: Query<&mut HtmlStyle>,
) {
    for (entity, parent, pressed, option) in options.iter() {
        if !pressed {
            continue;
        }

        events.write(SelectionChangedEvent {
            select: option.select,
            option: entity,
        });

        // close the list
        _ = styles.get_mut(parent.parent()).map(|mut style| {
            style.computed.node.display = Display::None;
        });
    }
}

fn update_selection(
    mut cmd: Commands,
    mut events: MessageReader<SelectionChangedEvent>,
    mut texts: Query<&mut Text>,
    children: Query<&Children>,
    tags: Query<&Tags>,
) {
    for event in events.read() {
        let Some(mut text) = children
            .get(event.select)
            .ok()
            .map(|children| {
                children
                    .iter()
                    .filter(|child| texts.get(*child).is_ok())
                    .next()
            })
            .flatten()
            .map(|c| texts.get_mut(c).ok())
            .flatten()
        else {
            continue;
        };

        _ = tags
            .get(event.option)
            .map(|tags| tags.get("value").map(|s| s.as_str()).unwrap_or_default())
            .map(|t| text.0 = t.into());

        cmd.trigger(UiChangedEvent {
            entity: event.select,
        });
    }
}
