use crate::{build::HtmlNode, data::HtmlTemplate};
use bevy::{
    ecs::system::{EntityCommands, SystemId, SystemParam},
    platform::collections::HashMap,
    prelude::*,
};
#[cfg(feature = "picking")]
use bevy::ui::Pressed;
#[cfg(feature = "picking")]
use bevy_picking::hover::Hovered;

pub struct BindingPlugin;
impl Plugin for BindingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FunctionBindings>()
            .init_resource::<ComponentBindings>()
            // .add_event::<UiChangedEvent>()
            .add_systems(
                Update,
                (
                    #[cfg(feature = "picking")]
                    observe_interactions,
                    observe_on_spawn,
                ),
            )
            .add_observer(observe_node_changed);
    }
}

/// A user triggered event to notify about a change. This
/// will trigger any attached [crate::build::OnUiChange]
/// via an entity obverser function binding.
///
/// In template `on_change="my_func_binging"`
///
/// commonly used to build widgets like sliders/input that should
/// react to any change
#[derive(Event)]
pub struct UiChangedEvent {
    pub entity: Entity,
}

pub type SpawnFunction = dyn Fn(EntityCommands) + Send + Sync + 'static;

#[derive(SystemParam)]
pub struct HtmlFunctions<'w, 's> {
    bindings: ResMut<'w, FunctionBindings>,
    cmd: Commands<'w, 's>,
}

impl<'w, 's> HtmlFunctions<'w, 's> {
    pub fn register<S, M>(&mut self, name: impl Into<String>, func: S)
    where
        S: IntoSystem<In<Entity>, (), M> + 'static,
    {
        let id = self.cmd.register_system(func);
        self.bindings.register(name, id);
    }
}

#[derive(SystemParam)]
pub struct HtmlComponents<'w> {
    comps: ResMut<'w, ComponentBindings>,
}

impl<'w> HtmlComponents<'w> {
    /// link any custom html node to your template
    pub fn register(&mut self, name: impl Into<String>, template: Handle<HtmlTemplate>) {
        self.comps.register(name, move |mut cmd| {
            cmd.insert(HtmlNode(template.clone()));
        });
    }

    /// takes a closure with acces to `EntityCommands`
    /// attach custom components on spawn
    pub fn register_with_spawn_fn<SF>(
        &mut self,
        name: impl Into<String>,
        template: Handle<HtmlTemplate>,
        func: SF,
    ) where
        SF: Fn(EntityCommands) + Send + Sync + 'static,
    {
        self.comps.register(name, move |mut cmd| {
            cmd.insert(HtmlNode(template.clone()));
            func(cmd);
        });
    }
}

/// # Register custom node tags
///
/// then use in your templats `<my_comp></my_comp>`
/// `
/// ComponenRegistry.register("my_comp", &|mut cmd: EntityCommands| cmd.insert(MyBundle::default()))
/// `
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ComponentBindings(HashMap<String, Box<SpawnFunction>>);

impl ComponentBindings {
    pub fn register<F>(&mut self, key: impl Into<String>, f: F)
    where
        F: Fn(EntityCommands) + Send + Sync + 'static,
    {
        let key: String = key.into();
        self.insert(key, Box::new(f));
    }

    pub fn try_spawn(&self, key: &String, entity: Entity, cmd: &mut Commands) {
        self.get(key)
            .map(|f| {
                let cmd = cmd.entity(entity);
                f(cmd);
            })
            .unwrap_or_else(|| warn!("custom tag `{key}` is not bound"));
    }
}

/// # Function binding resource
///
/// maps an oneshot system to a callable action, passing the Entity the action is
/// bound to.
///
/// in templates: `click="start_game"`
///
/// backend:
///
/// `
/// let system_id = app.register_system(|entity: In<Entity>| {})
/// FunctionBindings.register("start_game", system_id);
/// `
#[derive(Resource, Default, Deref, DerefMut, Debug)]
pub struct FunctionBindings(HashMap<String, SystemId<In<Entity>>>);

impl FunctionBindings {
    pub fn register(&mut self, key: impl Into<String>, system_id: SystemId<In<Entity>>) {
        let key: String = key.into();
        self.insert(key, system_id);
    }

    pub fn maybe_run(&self, key: &String, entity: Entity, cmd: &mut Commands) {
        self.get(key)
            .map(|id| {
                cmd.run_system_with(*id, entity);
            })
            .unwrap_or_else(|| warn!("function `{key}` is not bound"));
    }
}

fn observe_on_spawn(
    mut cmd: Commands,
    function_bindings: Res<FunctionBindings>,
    on_spawn: Query<(Entity, &crate::prelude::OnUiSpawn)>,
) {
    on_spawn.iter().for_each(|(entity, on_spawn)| {
        for spawn_fn in on_spawn.iter() {
            function_bindings.maybe_run(spawn_fn, entity, &mut cmd);
        }

        cmd.entity(entity).remove::<crate::prelude::OnUiSpawn>();
    });
}

#[rustfmt::skip]
#[cfg(feature = "picking")]
fn observe_interactions(
    mut cmd: Commands,
    pressed : Query<(Entity, &crate::prelude::OnUiPress), Added<Pressed>>,
    enters : Query<(Entity, &Hovered, &crate::prelude::OnUiEnter), Changed<Hovered>>,
    exits : Query<(Entity, &Hovered, &crate::prelude::OnUiExit), Changed<Hovered>>,
    function_bindings: Res<FunctionBindings>,
){
    pressed.iter().for_each(|(entity, on_pressed)|{
        for fn_str in on_pressed.iter(){
            function_bindings.maybe_run(fn_str, entity, &mut cmd);
        }
    });

    enters.iter().for_each(|(entity, hovered, on_enter)|{
        if !hovered.get() {
            return;
        }
        for fn_str in on_enter.iter(){
            function_bindings.maybe_run(fn_str, entity, &mut cmd);
        }
    });

    exits.iter().for_each(|(entity, hovered, on_exit)|{
        if hovered.get() {
            return;
        }
        for fn_str in on_exit.iter(){
            function_bindings.maybe_run(fn_str, entity, &mut cmd);
        }
    });
}

/// runs any attached `on_change` function when the user
/// triggers the [UiChangedEvent] on the target enttiy.
fn observe_node_changed(
    trigger: On<UiChangedEvent>,
    mut cmd: Commands,
    on_change: Query<&crate::prelude::OnUiChange>,
    function_bindings: Res<FunctionBindings>,
) {
    let entity = trigger.event().entity;

    let Ok(funcs) = on_change.get(entity) else {
        return;
    };

    for fn_str in funcs.iter() {
        function_bindings.maybe_run(fn_str, entity, &mut cmd);
    }
}
