#![expect(
    unsafe_op_in_unsafe_fn,
    reason = "See #11590. To be removed once all applicable unsafe code has an unsafe block with a safety comment."
)]
#![doc = include_str!("../README.md")]
#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(doc_auto_cfg, rustdoc_internals))]
#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(target_pointer_width = "16")]
compile_error!("bevy_ecs cannot safely compile for a 16-bit platform.");

extern crate alloc;

// Required to make proc macros work in bevy itself.
extern crate self as bevy_ecs;

pub mod additions;
pub mod packet;
pub mod archetype;
pub mod batching;
pub mod bundle;
pub mod change_detection;
pub mod component;
pub mod entity;
pub mod entity_disabling;
pub mod error;
pub mod event;
pub mod hierarchy;
pub mod identifier;
pub mod intern;
pub mod label;
pub mod name;
pub mod never;
pub mod observer;
pub mod query;
#[cfg(feature = "bevy_reflect")]
pub mod reflect;
pub mod relationship;
pub mod removal_detection;
pub mod resource;
pub mod schedule;
pub mod spawn;
pub mod storage;
pub mod system;
pub mod traversal;
pub mod world;

pub use bevy_ptr as ptr;

/// The ECS prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[expect(
        deprecated,
        reason = "`crate::schedule::apply_deferred` is considered deprecated; however, it may still be used by crates which consume `bevy_ecs`, so its removal here may cause confusion. It is intended to be removed in the Bevy 0.17 cycle."
    )]
    #[doc(hidden)]
    pub use crate::{
        bundle::Bundle,
        change_detection::{DetectChanges, DetectChangesMut, Mut, Ref},
        children,
        component::Component,
        entity::{ContainsEntity, Entity, EntityMapper},
        error::{BevyError, Result},
        event::{Event, EventMutator, EventReader, EventWriter, Events},
        hierarchy::{ChildOf, ChildSpawner, ChildSpawnerCommands, Children},
        name::{Name, NameOrEntity},
        observer::{Observer, Trigger},
        query::{Added, AnyOf, Changed, Has, Or, QueryBuilder, QueryState, With, Without},
        related,
        relationship::RelationshipTarget,
        removal_detection::RemovedComponents,
        resource::Resource,
        schedule::{
            apply_deferred, common_conditions::*, ApplyDeferred, Condition, IntoScheduleConfigs,
            IntoSystemSet, Schedule, Schedules, SystemSet,
        },
        spawn::{Spawn, SpawnRelated},
        system::{
            Command, Commands, Deferred, EntityCommand, EntityCommands, In, InMut, InRef,
            IntoSystem, Local, NonSend, NonSendMut, ParamSet, Populated, Query, ReadOnlySystem,
            Res, ResMut, Single, System, SystemIn, SystemInput, SystemParamBuilder,
            SystemParamFunction,
        },
        world::{
            EntityMut, EntityRef, EntityWorldMut, FilteredResources, FilteredResourcesMut,
            FromWorld, OnAdd, OnInsert, OnRemove, OnReplace, World,
        },
    };

    #[doc(hidden)]
    #[cfg(feature = "std")]
    pub use crate::system::ParallelCommands;

    #[doc(hidden)]
    #[cfg(feature = "bevy_reflect")]
    pub use crate::reflect::{
        AppTypeRegistry, ReflectComponent, ReflectFromWorld, ReflectResource,
    };

    #[doc(hidden)]
    #[cfg(feature = "reflect_functions")]
    pub use crate::reflect::AppFunctionRegistry;
}

/// Exports used by macros.
///
/// These are not meant to be used directly and are subject to breaking changes.
#[doc(hidden)]
pub mod __macro_exports {
    // Cannot directly use `alloc::vec::Vec` in macros, as a crate may not have
    // included `extern crate alloc;`. This re-export ensures we have access
    // to `Vec` in `no_std` and `std` contexts.
    pub use alloc::vec::Vec;
}

#[cfg(test)]
mod tests {
    use crate::{
        bundle::Bundle,
        change_detection::Ref,
        component::{Component, ComponentId, RequiredComponents, RequiredComponentsError},
        entity::{Entity, EntityMapper},
        entity_disabling::DefaultQueryFilters,
        prelude::Or,
        query::{Added, Changed, FilteredAccess, QueryFilter, With, Without},
        resource::Resource,
        world::{EntityMut, EntityRef, Mut, World},
    };
    use alloc::{
        string::{String, ToString},
        sync::Arc,
        vec,
        vec::Vec,
    };
    use bevy_platform::collections::HashSet;
    use bevy_tasks::{ComputeTaskPool, TaskPool};
    use core::{
        any::TypeId,
        marker::PhantomData,
        num::NonZero,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use std::sync::Mutex;

    #[derive(Component, Resource, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    struct A(usize);
    #[derive(Component, Debug, PartialEq, Eq, Hash, Clone, Copy)]
    struct B(usize);
    #[derive(Component, Debug, PartialEq, Eq, Clone, Copy)]
    struct C;

    #[derive(Default)]
    struct NonSendA(PhantomData<*mut ()>);

    #[derive(Component, Clone, Debug)]
    struct DropCk(Arc<AtomicUsize>);
    impl DropCk {
        fn new_pair() -> (Self, Arc<AtomicUsize>) {
            let atomic = Arc::new(AtomicUsize::new(0));
            (DropCk(atomic.clone()), atomic)
        }
    }

    impl Drop for DropCk {
        fn drop(&mut self) {
            self.0.as_ref().fetch_add(1, Ordering::Relaxed);
        }
    }

    #[expect(
        dead_code,
        reason = "This struct is used to test how `Drop` behavior works in regards to SparseSet storage, and as such is solely a wrapper around `DropCk` to make it use the SparseSet storage. Because of this, the inner field is intentionally never read."
    )]
    #[derive(Component, Clone, Debug)]
    #[component(storage = "SparseSet")]
    struct DropCkSparse(DropCk);

    #[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
    #[component(storage = "Table")]
    struct TableStored(&'static str);
    #[derive(Component, Copy, Clone, PartialEq, Eq, Hash, Debug)]
    #[component(storage = "SparseSet")]
    struct SparseStored(u32);

    #[test]
    fn random_access() {
        let mut world = World::new();

        let e = world.spawn((TableStored("abc"), SparseStored(123))).id();
        let f = world
            .spawn((TableStored("def"), SparseStored(456), A(1)))
            .id();
        assert_eq!(world.get::<TableStored>(e).unwrap().0, "abc");
        assert_eq!(world.get::<SparseStored>(e).unwrap().0, 123);
        assert_eq!(world.get::<TableStored>(f).unwrap().0, "def");
        assert_eq!(world.get::<SparseStored>(f).unwrap().0, 456);

        // test archetype get_mut()
        world.get_mut::<TableStored>(e).unwrap().0 = "xyz";
        assert_eq!(world.get::<TableStored>(e).unwrap().0, "xyz");

        // test sparse set get_mut()
        world.get_mut::<SparseStored>(f).unwrap().0 = 42;
        assert_eq!(world.get::<SparseStored>(f).unwrap().0, 42);
    }

    #[test]
    fn bundle_derive() {
        let mut world = World::new();

        #[derive(Bundle, PartialEq, Debug)]
        struct FooBundle {
            x: TableStored,
            y: SparseStored,
        }
        let mut ids = Vec::new();
        <FooBundle as Bundle>::component_ids(&mut world.components_registrator(), &mut |id| {
            ids.push(id);
        });

        assert_eq!(
            ids,
            &[
                world.register_component::<TableStored>(),
                world.register_component::<SparseStored>(),
            ]
        );

        let e1 = world
            .spawn(FooBundle {
                x: TableStored("abc"),
                y: SparseStored(123),
            })
            .id();
        let e2 = world
            .spawn((TableStored("def"), SparseStored(456), A(1)))
            .id();
        assert_eq!(world.get::<TableStored>(e1).unwrap().0, "abc");
        assert_eq!(world.get::<SparseStored>(e1).unwrap().0, 123);
        assert_eq!(world.get::<TableStored>(e2).unwrap().0, "def");
        assert_eq!(world.get::<SparseStored>(e2).unwrap().0, 456);

        // test archetype get_mut()
        world.get_mut::<TableStored>(e1).unwrap().0 = "xyz";
        assert_eq!(world.get::<TableStored>(e1).unwrap().0, "xyz");

        // test sparse set get_mut()
        world.get_mut::<SparseStored>(e2).unwrap().0 = 42;
        assert_eq!(world.get::<SparseStored>(e2).unwrap().0, 42);

        assert_eq!(
            world.entity_mut(e1).take::<FooBundle>().unwrap(),
            FooBundle {
                x: TableStored("xyz"),
                y: SparseStored(123),
            }
        );

        #[derive(Bundle, PartialEq, Debug)]
        struct NestedBundle {
            a: A,
            foo: FooBundle,
            b: B,
        }

        let mut ids = Vec::new();
        <NestedBundle as Bundle>::component_ids(&mut world.components_registrator(), &mut |id| {
            ids.push(id);
        });

        assert_eq!(
            ids,
            &[
                world.register_component::<A>(),
                world.register_component::<TableStored>(),
                world.register_component::<SparseStored>(),
                world.register_component::<B>(),
            ]
        );

        let e3 = world
            .spawn(NestedBundle {
                a: A(1),
                foo: FooBundle {
                    x: TableStored("ghi"),
                    y: SparseStored(789),
                },
                b: B(2),
            })
            .id();

        assert_eq!(world.get::<TableStored>(e3).unwrap().0, "ghi");
        assert_eq!(world.get::<SparseStored>(e3).unwrap().0, 789);
        assert_eq!(world.get::<A>(e3).unwrap().0, 1);
        assert_eq!(world.get::<B>(e3).unwrap().0, 2);
        assert_eq!(
            world.entity_mut(e3).take::<NestedBundle>().unwrap(),
            NestedBundle {
                a: A(1),
                foo: FooBundle {
                    x: TableStored("ghi"),
                    y: SparseStored(789),
                },
                b: B(2),
            }
        );

        #[derive(Default, Component, PartialEq, Debug)]
        struct Ignored;

        #[derive(Bundle, PartialEq, Debug)]
        struct BundleWithIgnored {
            c: C,
            #[bundle(ignore)]
            ignored: Ignored,
        }

        let mut ids = Vec::new();
        <BundleWithIgnored as Bundle>::component_ids(
            &mut world.components_registrator(),
            &mut |id| {
                ids.push(id);
            },
        );

        assert_eq!(ids, &[world.register_component::<C>(),]);

        let e4 = world
            .spawn(BundleWithIgnored {
                c: C,
                ignored: Ignored,
            })
            .id();

        assert_eq!(world.get::<C>(e4).unwrap(), &C);
        assert_eq!(world.get::<Ignored>(e4), None);

        assert_eq!(
            world.entity_mut(e4).take::<BundleWithIgnored>().unwrap(),
            BundleWithIgnored {
                c: C,
                ignored: Ignored,
            }
        );
    }

    #[test]
    fn despawn_table_storage() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world.spawn((TableStored("def"), A(456))).id();
        assert_eq!(world.entities.len(), 2);
        assert!(world.despawn(e));
        assert_eq!(world.entities.len(), 1);
        assert!(world.get::<TableStored>(e).is_none());
        assert!(world.get::<A>(e).is_none());
        assert_eq!(world.get::<TableStored>(f).unwrap().0, "def");
        assert_eq!(world.get::<A>(f).unwrap().0, 456);
    }

    #[test]
    fn despawn_mixed_storage() {
        let mut world = World::new();

        let e = world.spawn((TableStored("abc"), SparseStored(123))).id();
        let f = world.spawn((TableStored("def"), SparseStored(456))).id();
        assert_eq!(world.entities.len(), 2);
        assert!(world.despawn(e));
        assert_eq!(world.entities.len(), 1);
        assert!(world.get::<TableStored>(e).is_none());
        assert!(world.get::<SparseStored>(e).is_none());
        assert_eq!(world.get::<TableStored>(f).unwrap().0, "def");
        assert_eq!(world.get::<SparseStored>(f).unwrap().0, 456);
    }

    #[test]
    fn query_all() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world.spawn((TableStored("def"), A(456))).id();

        let ents = world
            .query::<(Entity, &A, &TableStored)>()
            .iter(&world)
            .map(|(e, &i, &s)| (e, i, s))
            .collect::<Vec<_>>();
        assert_eq!(
            ents,
            &[
                (e, A(123), TableStored("abc")),
                (f, A(456), TableStored("def"))
            ]
        );
    }

    #[test]
    fn query_all_for_each() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world.spawn((TableStored("def"), A(456))).id();

        let mut results = Vec::new();
        world
            .query::<(Entity, &A, &TableStored)>()
            .iter(&world)
            .for_each(|(e, &i, &s)| results.push((e, i, s)));
        assert_eq!(
            results,
            &[
                (e, A(123), TableStored("abc")),
                (f, A(456), TableStored("def"))
            ]
        );
    }

    #[test]
    fn query_single_component() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world.spawn((TableStored("def"), A(456), B(1))).id();
        let ents = world
            .query::<(Entity, &A)>()
            .iter(&world)
            .map(|(e, &i)| (e, i))
            .collect::<HashSet<_>>();
        assert!(ents.contains(&(e, A(123))));
        assert!(ents.contains(&(f, A(456))));
    }

    #[test]
    fn stateful_query_handles_new_archetype() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        let mut query = world.query::<(Entity, &A)>();

        let ents = query.iter(&world).map(|(e, &i)| (e, i)).collect::<Vec<_>>();
        assert_eq!(ents, &[(e, A(123))]);

        let f = world.spawn((TableStored("def"), A(456), B(1))).id();
        let ents = query.iter(&world).map(|(e, &i)| (e, i)).collect::<Vec<_>>();
        assert_eq!(ents, &[(e, A(123)), (f, A(456))]);
    }

    #[test]
    fn query_single_component_for_each() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world.spawn((TableStored("def"), A(456), B(1))).id();
        let mut results = <HashSet<_>>::default();
        world
            .query::<(Entity, &A)>()
            .iter(&world)
            .for_each(|(e, &i)| {
                results.insert((e, i));
            });
        assert!(results.contains(&(e, A(123))));
        assert!(results.contains(&(f, A(456))));
    }

    #[test]
    fn par_for_each_dense() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::new();
        let e1 = world.spawn(A(1)).id();
        let e2 = world.spawn(A(2)).id();
        let e3 = world.spawn(A(3)).id();
        let e4 = world.spawn((A(4), B(1))).id();
        let e5 = world.spawn((A(5), B(1))).id();
        let results = Arc::new(Mutex::new(Vec::new()));
        world
            .query::<(Entity, &A)>()
            .par_iter(&world)
            .for_each(|(e, &A(i))| {
                results.lock().unwrap().push((e, i));
            });
        results.lock().unwrap().sort();
        assert_eq!(
            &*results.lock().unwrap(),
            &[(e1, 1), (e2, 2), (e3, 3), (e4, 4), (e5, 5)]
        );
    }

    #[test]
    fn par_for_each_sparse() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::new();
        let e1 = world.spawn(SparseStored(1)).id();
        let e2 = world.spawn(SparseStored(2)).id();
        let e3 = world.spawn(SparseStored(3)).id();
        let e4 = world.spawn((SparseStored(4), A(1))).id();
        let e5 = world.spawn((SparseStored(5), A(1))).id();
        let results = Arc::new(Mutex::new(Vec::new()));
        world
            .query::<(Entity, &SparseStored)>()
            .par_iter(&world)
            .for_each(|(e, &SparseStored(i))| results.lock().unwrap().push((e, i)));
        results.lock().unwrap().sort();
        assert_eq!(
            &*results.lock().unwrap(),
            &[(e1, 1), (e2, 2), (e3, 3), (e4, 4), (e5, 5)]
        );
    }

    #[test]
    fn query_missing_component() {
        let mut world = World::new();
        world.spawn((TableStored("abc"), A(123)));
        world.spawn((TableStored("def"), A(456)));
        assert!(world.query::<(&B, &A)>().iter(&world).next().is_none());
    }

    #[test]
    fn query_sparse_component() {
        let mut world = World::new();
        world.spawn((TableStored("abc"), A(123)));
        let f = world.spawn((TableStored("def"), A(456), B(1))).id();
        let ents = world
            .query::<(Entity, &B)>()
            .iter(&world)
            .map(|(e, &b)| (e, b))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(f, B(1))]);
    }

    #[test]
    fn query_filter_with() {
        let mut world = World::new();
        world.spawn((A(123), B(1)));
        world.spawn(A(456));
        let result = world
            .query_filtered::<&A, With<B>>()
            .iter(&world)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(result, vec![A(123)]);
    }

    #[test]
    fn query_filter_with_for_each() {
        let mut world = World::new();
        world.spawn((A(123), B(1)));
        world.spawn(A(456));

        let mut results = Vec::new();
        world
            .query_filtered::<&A, With<B>>()
            .iter(&world)
            .for_each(|i| results.push(*i));
        assert_eq!(results, vec![A(123)]);
    }

    #[test]
    fn query_filter_with_sparse() {
        let mut world = World::new();

        world.spawn((A(123), SparseStored(321)));
        world.spawn(A(456));
        let result = world
            .query_filtered::<&A, With<SparseStored>>()
            .iter(&world)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(result, vec![A(123)]);
    }

    #[test]
    fn query_filter_with_sparse_for_each() {
        let mut world = World::new();

        world.spawn((A(123), SparseStored(321)));
        world.spawn(A(456));
        let mut results = Vec::new();
        world
            .query_filtered::<&A, With<SparseStored>>()
            .iter(&world)
            .for_each(|i| results.push(*i));
        assert_eq!(results, vec![A(123)]);
    }

    #[test]
    fn query_filter_without() {
        let mut world = World::new();
        world.spawn((A(123), B(321)));
        world.spawn(A(456));
        let result = world
            .query_filtered::<&A, Without<B>>()
            .iter(&world)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(result, vec![A(456)]);
    }

    #[test]
    fn query_optional_component_table() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world.spawn((TableStored("def"), A(456), B(1))).id();
        // this should be skipped
        world.spawn(TableStored("abc"));
        let ents = world
            .query::<(Entity, Option<&B>, &A)>()
            .iter(&world)
            .map(|(e, b, &i)| (e, b.copied(), i))
            .collect::<HashSet<_>>();
        assert!(ents.contains(&(e, None, A(123))));
        assert!(ents.contains(&(f, Some(B(1)), A(456))));
    }

    #[test]
    fn query_optional_component_sparse() {
        let mut world = World::new();

        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world
            .spawn((TableStored("def"), A(456), SparseStored(1)))
            .id();
        // this should be skipped
        // world.spawn(SparseStored(1));
        let ents = world
            .query::<(Entity, Option<&SparseStored>, &A)>()
            .iter(&world)
            .map(|(e, b, &i)| (e, b.copied(), i))
            .collect::<HashSet<_>>();
        assert_eq!(
            ents,
            [(e, None, A(123)), (f, Some(SparseStored(1)), A(456))]
                .into_iter()
                .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn query_optional_component_sparse_no_match() {
        let mut world = World::new();

        let e = world.spawn((TableStored("abc"), A(123))).id();
        let f = world.spawn((TableStored("def"), A(456))).id();
        // // this should be skipped
        world.spawn(TableStored("abc"));
        let ents = world
            .query::<(Entity, Option<&SparseStored>, &A)>()
            .iter(&world)
            .map(|(e, b, &i)| (e, b.copied(), i))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(e, None, A(123)), (f, None, A(456))]);
    }

    #[test]
    fn add_remove_components() {
        let mut world = World::new();
        let e1 = world.spawn((A(1), B(3), TableStored("abc"))).id();
        let e2 = world.spawn((A(2), B(4), TableStored("xyz"))).id();

        assert_eq!(
            world
                .query::<(Entity, &A, &B)>()
                .iter(&world)
                .map(|(e, &i, &b)| (e, i, b))
                .collect::<HashSet<_>>(),
            [(e1, A(1), B(3)), (e2, A(2), B(4))]
                .into_iter()
                .collect::<HashSet<_>>()
        );
        assert_eq!(world.entity_mut(e1).take::<A>(), Some(A(1)));
        assert_eq!(
            world
                .query::<(Entity, &A, &B)>()
                .iter(&world)
                .map(|(e, &i, &b)| (e, i, b))
                .collect::<Vec<_>>(),
            &[(e2, A(2), B(4))]
        );
        assert_eq!(
            world
                .query::<(Entity, &B, &TableStored)>()
                .iter(&world)
                .map(|(e, &B(b), &TableStored(s))| (e, b, s))
                .collect::<HashSet<_>>(),
            [(e2, 4, "xyz"), (e1, 3, "abc")]
                .into_iter()
                .collect::<HashSet<_>>()
        );
        world.entity_mut(e1).insert(A(43));
        assert_eq!(
            world
                .query::<(Entity, &A, &B)>()
                .iter(&world)
                .map(|(e, &i, &b)| (e, i, b))
                .collect::<HashSet<_>>(),
            [(e2, A(2), B(4)), (e1, A(43), B(3))]
                .into_iter()
                .collect::<HashSet<_>>()
        );
        world.entity_mut(e1).insert(C);
        assert_eq!(
            world
                .query::<(Entity, &C)>()
                .iter(&world)
                .map(|(e, &f)| (e, f))
                .collect::<Vec<_>>(),
            &[(e1, C)]
        );
    }

    #[test]
    fn table_add_remove_many() {
        let mut world = World::default();
        #[cfg(miri)]
        let (mut entities, to) = {
            let to = 10;
            (Vec::with_capacity(to), to)
        };
        #[cfg(not(miri))]
        let (mut entities, to) = {
            let to = 10_000;
            (Vec::with_capacity(to), to)
        };

        for _ in 0..to {
            entities.push(world.spawn(B(0)).id());
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            world.entity_mut(entity).insert(A(i));
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            assert_eq!(world.entity_mut(entity).take::<A>(), Some(A(i)));
        }
    }

    #[test]
    fn sparse_set_add_remove_many() {
        let mut world = World::default();

        let mut entities = Vec::with_capacity(1000);
        for _ in 0..4 {
            entities.push(world.spawn(A(2)).id());
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            world.entity_mut(entity).insert(SparseStored(i as u32));
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            assert_eq!(
                world.entity_mut(entity).take::<SparseStored>(),
                Some(SparseStored(i as u32))
            );
        }
    }

    #[test]
    fn remove_missing() {
        let mut world = World::new();
        let e = world.spawn((TableStored("abc"), A(123))).id();
        assert!(world.entity_mut(e).take::<B>().is_none());
    }

    #[test]
    fn spawn_batch() {
        let mut world = World::new();
        world.spawn_batch((0..100).map(|x| (A(x), TableStored("abc"))));
        let values = world
            .query::<&A>()
            .iter(&world)
            .map(|v| v.0)
            .collect::<Vec<_>>();
        let expected = (0..100).collect::<Vec<_>>();
        assert_eq!(values, expected);
    }

    #[test]
    fn query_get() {
        let mut world = World::new();
        let a = world.spawn((TableStored("abc"), A(123))).id();
        let b = world.spawn((TableStored("def"), A(456))).id();
        let c = world.spawn((TableStored("ghi"), A(789), B(1))).id();

        let mut i32_query = world.query::<&A>();
        assert_eq!(i32_query.get(&world, a).unwrap().0, 123);
        assert_eq!(i32_query.get(&world, b).unwrap().0, 456);

        let mut i32_bool_query = world.query::<(&A, &B)>();
        assert!(i32_bool_query.get(&world, a).is_err());
        assert_eq!(i32_bool_query.get(&world, c).unwrap(), (&A(789), &B(1)));
        assert!(world.despawn(a));
        assert!(i32_query.get(&world, a).is_err());
    }

    #[test]
    fn query_get_works_across_sparse_removal() {
        // Regression test for: https://github.com/bevyengine/bevy/issues/6623
        let mut world = World::new();
        let a = world.spawn((TableStored("abc"), SparseStored(123))).id();
        let b = world.spawn((TableStored("def"), SparseStored(456))).id();
        let c = world
            .spawn((TableStored("ghi"), SparseStored(789), B(1)))
            .id();

        let mut query = world.query::<&TableStored>();
        assert_eq!(query.get(&world, a).unwrap(), &TableStored("abc"));
        assert_eq!(query.get(&world, b).unwrap(), &TableStored("def"));
        assert_eq!(query.get(&world, c).unwrap(), &TableStored("ghi"));

        world.entity_mut(b).remove::<SparseStored>();
        world.entity_mut(c).remove::<SparseStored>();

        assert_eq!(query.get(&world, a).unwrap(), &TableStored("abc"));
        assert_eq!(query.get(&world, b).unwrap(), &TableStored("def"));
        assert_eq!(query.get(&world, c).unwrap(), &TableStored("ghi"));
    }

    #[test]
    fn remove_tracking() {
        let mut world = World::new();

        let a = world.spawn((SparseStored(0), A(123))).id();
        let b = world.spawn((SparseStored(1), A(123))).id();

        world.entity_mut(a).despawn();
        assert_eq!(
            world.removed::<A>().collect::<Vec<_>>(),
            &[a],
            "despawning results in 'removed component' state for table components"
        );
        assert_eq!(
            world.removed::<SparseStored>().collect::<Vec<_>>(),
            &[a],
            "despawning results in 'removed component' state for sparse set components"
        );

        world.entity_mut(b).insert(B(1));
        assert_eq!(
            world.removed::<A>().collect::<Vec<_>>(),
            &[a],
            "archetype moves does not result in 'removed component' state"
        );

        world.entity_mut(b).remove::<A>();
        assert_eq!(
            world.removed::<A>().collect::<Vec<_>>(),
            &[a, b],
            "removing a component results in a 'removed component' state"
        );

        world.clear_trackers();
        assert_eq!(
            world.removed::<A>().collect::<Vec<_>>(),
            &[],
            "clearing trackers clears removals"
        );
        assert_eq!(
            world.removed::<SparseStored>().collect::<Vec<_>>(),
            &[],
            "clearing trackers clears removals"
        );
        assert_eq!(
            world.removed::<B>().collect::<Vec<_>>(),
            &[],
            "clearing trackers clears removals"
        );

        // TODO: uncomment when world.clear() is implemented
        // let c = world.spawn(("abc", 123)).id();
        // let d = world.spawn(("abc", 123)).id();
        // world.clear();
        // assert_eq!(
        //     world.removed::<i32>(),
        //     &[c, d],
        //     "world clears result in 'removed component' states"
        // );
        // assert_eq!(
        //     world.removed::<&'static str>(),
        //     &[c, d, b],
        //     "world clears result in 'removed component' states"
        // );
        // assert_eq!(
        //     world.removed::<f64>(),
        //     &[b],
        //     "world clears result in 'removed component' states"
        // );
    }

    #[test]
    fn added_tracking() {
        let mut world = World::new();
        let a = world.spawn(A(123)).id();

        assert_eq!(world.query::<&A>().iter(&world).count(), 1);
        assert_eq!(
            world.query_filtered::<(), Added<A>>().iter(&world).count(),
            1
        );
        assert_eq!(world.query::<&A>().iter(&world).count(), 1);
        assert_eq!(
            world.query_filtered::<(), Added<A>>().iter(&world).count(),
            1
        );
        assert!(world.query::<&A>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<A>>()
            .get(&world, a)
            .is_ok());
        assert!(world.query::<&A>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<A>>()
            .get(&world, a)
            .is_ok());

        world.clear_trackers();

        assert_eq!(world.query::<&A>().iter(&world).count(), 1);
        assert_eq!(
            world.query_filtered::<(), Added<A>>().iter(&world).count(),
            0
        );
        assert_eq!(world.query::<&A>().iter(&world).count(), 1);
        assert_eq!(
            world.query_filtered::<(), Added<A>>().iter(&world).count(),
            0
        );
        assert!(world.query::<&A>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<A>>()
            .get(&world, a)
            .is_err());
        assert!(world.query::<&A>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<A>>()
            .get(&world, a)
            .is_err());
    }

    #[test]
    fn added_queries() {
        let mut world = World::default();
        let e1 = world.spawn(A(0)).id();

        fn get_added<Com: Component>(world: &mut World) -> Vec<Entity> {
            world
                .query_filtered::<Entity, Added<Com>>()
                .iter(world)
                .collect::<Vec<Entity>>()
        }

        assert_eq!(get_added::<A>(&mut world), vec![e1]);
        world.entity_mut(e1).insert(B(0));
        assert_eq!(get_added::<A>(&mut world), vec![e1]);
        assert_eq!(get_added::<B>(&mut world), vec![e1]);

        world.clear_trackers();
        assert!(get_added::<A>(&mut world).is_empty());
        let e2 = world.spawn((A(1), B(1))).id();
        assert_eq!(get_added::<A>(&mut world), vec![e2]);
        assert_eq!(get_added::<B>(&mut world), vec![e2]);

        let added = world
            .query_filtered::<Entity, (Added<A>, Added<B>)>()
            .iter(&world)
            .collect::<Vec<Entity>>();
        assert_eq!(added, vec![e2]);
    }

    #[test]
    fn changed_trackers() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B(0))).id();
        let e2 = world.spawn((A(0), B(0))).id();
        let e3 = world.spawn((A(0), B(0))).id();
        world.spawn((A(0), B(0)));

        world.clear_trackers();

        for (i, mut a) in world.query::<&mut A>().iter_mut(&mut world).enumerate() {
            if i % 2 == 0 {
                a.0 += 1;
            }
        }

        fn get_filtered<F: QueryFilter>(world: &mut World) -> HashSet<Entity> {
            world
                .query_filtered::<Entity, F>()
                .iter(world)
                .collect::<HashSet<Entity>>()
        }

        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e1, e3].into_iter().collect::<HashSet<_>>()
        );

        // ensure changing an entity's archetypes also moves its changed state
        world.entity_mut(e1).insert(C);

        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e3, e1].into_iter().collect::<HashSet<_>>(),
            "changed entities list should not change"
        );

        // spawning a new A entity should not change existing changed state
        world.entity_mut(e1).insert((A(0), B(0)));

        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e3, e1].into_iter().collect::<HashSet<_>>(),
            "changed entities list should not change"
        );

        // removing an unchanged entity should not change changed state
        assert!(world.despawn(e2));
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e3, e1].into_iter().collect::<HashSet<_>>(),
            "changed entities list should not change"
        );

        // removing a changed entity should remove it from enumeration
        assert!(world.despawn(e1));
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e3].into_iter().collect::<HashSet<_>>(),
            "e1 should no longer be returned"
        );

        world.clear_trackers();

        assert!(get_filtered::<Changed<A>>(&mut world).is_empty());

        let e4 = world.spawn_empty().id();

        world.entity_mut(e4).insert(A(0));
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            get_filtered::<Added<A>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );

        world.entity_mut(e4).insert(A(1));
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );

        world.clear_trackers();

        // ensure inserting multiple components set changed state for all components and set added
        // state for non existing components even when changing archetype.
        world.entity_mut(e4).insert((A(0), B(0)));

        assert!(get_filtered::<Added<A>>(&mut world).is_empty());
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            get_filtered::<Added<B>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            get_filtered::<Changed<B>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn changed_trackers_sparse() {
        let mut world = World::default();
        let e1 = world.spawn(SparseStored(0)).id();
        let e2 = world.spawn(SparseStored(0)).id();
        let e3 = world.spawn(SparseStored(0)).id();
        world.spawn(SparseStored(0));

        world.clear_trackers();

        for (i, mut a) in world
            .query::<&mut SparseStored>()
            .iter_mut(&mut world)
            .enumerate()
        {
            if i % 2 == 0 {
                a.0 += 1;
            }
        }

        fn get_filtered<F: QueryFilter>(world: &mut World) -> HashSet<Entity> {
            world
                .query_filtered::<Entity, F>()
                .iter(world)
                .collect::<HashSet<Entity>>()
        }

        assert_eq!(
            get_filtered::<Changed<SparseStored>>(&mut world),
            [e1, e3].into_iter().collect::<HashSet<_>>()
        );

        // ensure changing an entity's archetypes also moves its changed state
        world.entity_mut(e1).insert(C);

        assert_eq!(get_filtered::<Changed<SparseStored>>(&mut world), [e3, e1].into_iter().collect::<HashSet<_>>(), "changed entities list should not change (although the order will due to archetype moves)");

        // spawning a new SparseStored entity should not change existing changed state
        world.entity_mut(e1).insert(SparseStored(0));
        assert_eq!(
            get_filtered::<Changed<SparseStored>>(&mut world),
            [e3, e1].into_iter().collect::<HashSet<_>>(),
            "changed entities list should not change"
        );

        // removing an unchanged entity should not change changed state
        assert!(world.despawn(e2));
        assert_eq!(
            get_filtered::<Changed<SparseStored>>(&mut world),
            [e3, e1].into_iter().collect::<HashSet<_>>(),
            "changed entities list should not change"
        );

        // removing a changed entity should remove it from enumeration
        assert!(world.despawn(e1));
        assert_eq!(
            get_filtered::<Changed<SparseStored>>(&mut world),
            [e3].into_iter().collect::<HashSet<_>>(),
            "e1 should no longer be returned"
        );

        world.clear_trackers();

        assert!(get_filtered::<Changed<SparseStored>>(&mut world).is_empty());

        let e4 = world.spawn_empty().id();

        world.entity_mut(e4).insert(SparseStored(0));
        assert_eq!(
            get_filtered::<Changed<SparseStored>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            get_filtered::<Added<SparseStored>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );

        world.entity_mut(e4).insert(A(1));
        assert_eq!(
            get_filtered::<Changed<SparseStored>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );

        world.clear_trackers();

        // ensure inserting multiple components set changed state for all components and set added
        // state for non existing components even when changing archetype.
        world.entity_mut(e4).insert(SparseStored(0));

        assert!(get_filtered::<Added<SparseStored>>(&mut world).is_empty());
        assert_eq!(
            get_filtered::<Changed<SparseStored>>(&mut world),
            [e4].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn empty_spawn() {
        let mut world = World::default();
        let e = world.spawn_empty().id();
        let mut e_mut = world.entity_mut(e);
        e_mut.insert(A(0));
        assert_eq!(e_mut.get::<A>().unwrap(), &A(0));
    }

    #[test]
    fn reserve_and_spawn() {
        let mut world = World::default();
        let e = world.entities().reserve_entity();
        world.flush_entities();
        let mut e_mut = world.entity_mut(e);
        e_mut.insert(A(0));
        assert_eq!(e_mut.get::<A>().unwrap(), &A(0));
    }

    #[test]
    fn changed_query() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B(0))).id();

        fn get_changed(world: &mut World) -> Vec<Entity> {
            world
                .query_filtered::<Entity, Changed<A>>()
                .iter(world)
                .collect::<Vec<Entity>>()
        }
        assert_eq!(get_changed(&mut world), vec![e1]);
        world.clear_trackers();
        assert_eq!(get_changed(&mut world), vec![]);
        *world.get_mut(e1).unwrap() = A(1);
        assert_eq!(get_changed(&mut world), vec![e1]);
    }

    #[test]
    fn resource() {
        use crate::resource::Resource;

        #[derive(Resource, PartialEq, Debug)]
        struct Num(i32);

        #[derive(Resource, PartialEq, Debug)]
        struct BigNum(u64);

        let mut world = World::default();
        assert!(world.get_resource::<Num>().is_none());
        assert!(!world.contains_resource::<Num>());
        assert!(!world.is_resource_added::<Num>());
        assert!(!world.is_resource_changed::<Num>());

        world.insert_resource(Num(123));
        let resource_id = world
            .components()
            .get_resource_id(TypeId::of::<Num>())
            .unwrap();
        let archetype_component_id = world.storages().resources.get(resource_id).unwrap().id();

        assert_eq!(world.resource::<Num>().0, 123);
        assert!(world.contains_resource::<Num>());
        assert!(world.is_resource_added::<Num>());
        assert!(world.is_resource_changed::<Num>());

        world.insert_resource(BigNum(456));
        assert_eq!(world.resource::<BigNum>().0, 456u64);

        world.insert_resource(BigNum(789));
        assert_eq!(world.resource::<BigNum>().0, 789);

        {
            let mut value = world.resource_mut::<BigNum>();
            assert_eq!(value.0, 789);
            value.0 = 10;
        }

        assert_eq!(
            world.resource::<BigNum>().0,
            10,
            "resource changes are preserved"
        );

        assert_eq!(
            world.remove_resource::<BigNum>(),
            Some(BigNum(10)),
            "removed resource has the correct value"
        );
        assert_eq!(
            world.get_resource::<BigNum>(),
            None,
            "removed resource no longer exists"
        );
        assert_eq!(
            world.remove_resource::<BigNum>(),
            None,
            "double remove returns nothing"
        );

        world.insert_resource(BigNum(1));
        assert_eq!(
            world.get_resource::<BigNum>(),
            Some(&BigNum(1)),
            "re-inserting resources works"
        );

        assert_eq!(
            world.get_resource::<Num>(),
            Some(&Num(123)),
            "other resources are unaffected"
        );

        let current_resource_id = world
            .components()
            .get_resource_id(TypeId::of::<Num>())
            .unwrap();
        assert_eq!(
            resource_id, current_resource_id,
            "resource id does not change after removing / re-adding"
        );

        let current_archetype_component_id =
            world.storages().resources.get(resource_id).unwrap().id();

        assert_eq!(
            archetype_component_id, current_archetype_component_id,
            "resource archetype component id does not change after removing / re-adding"
        );
    }

    #[test]
    fn remove() {
        let mut world = World::default();
        let e1 = world.spawn((A(1), B(1), TableStored("a"))).id();

        let mut e = world.entity_mut(e1);
        assert_eq!(e.get::<TableStored>(), Some(&TableStored("a")));
        assert_eq!(e.get::<A>(), Some(&A(1)));
        assert_eq!(e.get::<B>(), Some(&B(1)));
        assert_eq!(
            e.get::<C>(),
            None,
            "C is not in the entity, so it should not exist"
        );

        e.remove::<(A, B, C)>();
        assert_eq!(
            e.get::<TableStored>(),
            Some(&TableStored("a")),
            "TableStored is not in the removed bundle, so it should exist"
        );
        assert_eq!(
            e.get::<A>(),
            None,
            "Num is in the removed bundle, so it should not exist"
        );
        assert_eq!(
            e.get::<B>(),
            None,
            "f64 is in the removed bundle, so it should not exist"
        );
        assert_eq!(
            e.get::<C>(),
            None,
            "usize is in the removed bundle, so it should not exist"
        );
    }

    #[test]
    fn take() {
        let mut world = World::default();
        world.spawn((A(1), B(1), TableStored("1")));
        let e2 = world.spawn((A(2), B(2), TableStored("2"))).id();
        world.spawn((A(3), B(3), TableStored("3")));

        let mut query = world.query::<(&B, &TableStored)>();
        let results = query
            .iter(&world)
            .map(|(a, b)| (a.0, b.0))
            .collect::<HashSet<_>>();
        assert_eq!(
            results,
            [(1, "1"), (2, "2"), (3, "3"),]
                .into_iter()
                .collect::<HashSet<_>>()
        );

        let removed_bundle = world.entity_mut(e2).take::<(B, TableStored)>().unwrap();
        assert_eq!(removed_bundle, (B(2), TableStored("2")));

        let results = query
            .iter(&world)
            .map(|(a, b)| (a.0, b.0))
            .collect::<HashSet<_>>();
        assert_eq!(
            results,
            [(1, "1"), (3, "3"),].into_iter().collect::<HashSet<_>>()
        );

        let mut a_query = world.query::<&A>();
        let results = a_query.iter(&world).map(|a| a.0).collect::<HashSet<_>>();
        assert_eq!(results, [1, 3, 2].into_iter().collect::<HashSet<_>>());

        let entity_ref = world.entity(e2);
        assert_eq!(
            entity_ref.get::<A>(),
            Some(&A(2)),
            "A is not in the removed bundle, so it should exist"
        );
        assert_eq!(
            entity_ref.get::<B>(),
            None,
            "B is in the removed bundle, so it should not exist"
        );
        assert_eq!(
            entity_ref.get::<TableStored>(),
            None,
            "TableStored is in the removed bundle, so it should not exist"
        );
    }

    #[test]
    fn non_send_resource() {
        let mut world = World::default();
        world.insert_non_send_resource(123i32);
        world.insert_non_send_resource(456i64);
        assert_eq!(*world.non_send_resource::<i32>(), 123);
        assert_eq!(*world.non_send_resource_mut::<i64>(), 456);
    }

    #[test]
    fn non_send_resource_points_to_distinct_data() {
        let mut world = World::default();
        world.insert_resource(A(123));
        world.insert_non_send_resource(A(456));
        assert_eq!(*world.resource::<A>(), A(123));
        assert_eq!(*world.non_send_resource::<A>(), A(456));
    }

    #[test]
    #[should_panic]
    fn non_send_resource_panic() {
        let mut world = World::default();
        world.insert_non_send_resource(0i32);
        std::thread::spawn(move || {
            let _ = world.non_send_resource_mut::<i32>();
        })
        .join()
        .unwrap();
    }

    #[test]
    fn exact_size_query() {
        let mut world = World::default();
        world.spawn((A(0), B(0)));
        world.spawn((A(0), B(0)));
        world.spawn((A(0), B(0), C));
        world.spawn(C);

        let mut query = world.query::<(&A, &B)>();
        assert_eq!(query.iter(&world).len(), 3);
    }

    #[test]
    #[should_panic]
    fn duplicate_components_panic() {
        let mut world = World::new();
        world.spawn((A(1), A(2)));
    }

    #[test]
    #[should_panic]
    fn ref_and_mut_query_panic() {
        let mut world = World::new();
        world.query::<(&A, &mut A)>();
    }

    #[test]
    #[should_panic]
    fn entity_ref_and_mut_query_panic() {
        let mut world = World::new();
        world.query::<(EntityRef, &mut A)>();
    }

    #[test]
    #[should_panic]
    fn mut_and_ref_query_panic() {
        let mut world = World::new();
        world.query::<(&mut A, &A)>();
    }

    #[test]
    #[should_panic]
    fn mut_and_entity_ref_query_panic() {
        let mut world = World::new();
        world.query::<(&mut A, EntityRef)>();
    }

    #[test]
    #[should_panic]
    fn entity_ref_and_entity_mut_query_panic() {
        let mut world = World::new();
        world.query::<(EntityRef, EntityMut)>();
    }

    #[test]
    #[should_panic]
    fn entity_mut_and_entity_mut_query_panic() {
        let mut world = World::new();
        world.query::<(EntityMut, EntityMut)>();
    }

    #[test]
    fn entity_ref_and_entity_ref_query_no_panic() {
        let mut world = World::new();
        world.query::<(EntityRef, EntityRef)>();
    }

    #[test]
    #[should_panic]
    fn mut_and_mut_query_panic() {
        let mut world = World::new();
        world.query::<(&mut A, &mut A)>();
    }

    #[test]
    #[should_panic]
    fn multiple_worlds_same_query_iter() {
        let mut world_a = World::new();
        let world_b = World::new();
        let mut query = world_a.query::<&A>();
        query.iter(&world_a);
        query.iter(&world_b);
    }

    #[test]
    fn query_filters_dont_collide_with_fetches() {
        let mut world = World::new();
        world.query_filtered::<&mut A, Changed<A>>();
    }

    #[test]
    fn filtered_query_access() {
        let mut world = World::new();
        // We remove entity disabling so it doesn't affect our query filters
        world.remove_resource::<DefaultQueryFilters>();
        let query = world.query_filtered::<&mut A, Changed<B>>();

        let mut expected = FilteredAccess::<ComponentId>::default();
        let a_id = world.components.get_id(TypeId::of::<A>()).unwrap();
        let b_id = world.components.get_id(TypeId::of::<B>()).unwrap();
        expected.add_component_write(a_id);
        expected.add_component_read(b_id);
        assert!(
            query.component_access.eq(&expected),
            "ComponentId access from query fetch and query filter should be combined"
        );
    }

    #[test]
    #[should_panic]
    fn multiple_worlds_same_query_get() {
        let mut world_a = World::new();
        let world_b = World::new();
        let mut query = world_a.query::<&A>();
        let _ = query.get(&world_a, Entity::from_raw(0));
        let _ = query.get(&world_b, Entity::from_raw(0));
    }

    #[test]
    #[should_panic]
    fn multiple_worlds_same_query_for_each() {
        let mut world_a = World::new();
        let world_b = World::new();
        let mut query = world_a.query::<&A>();
        query.iter(&world_a).for_each(|_| {});
        query.iter(&world_b).for_each(|_| {});
    }

    #[test]
    fn resource_scope() {
        let mut world = World::default();
        assert!(world.try_resource_scope::<A, _>(|_, _| {}).is_none());
        world.insert_resource(A(0));
        world.resource_scope(|world: &mut World, mut value: Mut<A>| {
            value.0 += 1;
            assert!(!world.contains_resource::<A>());
        });
        assert_eq!(world.resource::<A>().0, 1);
    }

    #[test]
    #[should_panic(
        expected = "Attempted to access or drop non-send resource bevy_ecs::tests::NonSendA from thread"
    )]
    fn non_send_resource_drop_from_different_thread() {
        let mut world = World::default();
        world.insert_non_send_resource(NonSendA::default());

        let thread = std::thread::spawn(move || {
            // Dropping the non-send resource on a different thread
            // Should result in a panic
            drop(world);
        });

        if let Err(err) = thread.join() {
            std::panic::resume_unwind(err);
        }
    }

    #[test]
    fn non_send_resource_drop_from_same_thread() {
        let mut world = World::default();
        world.insert_non_send_resource(NonSendA::default());
        drop(world);
    }

    #[test]
    fn insert_overwrite_drop() {
        let (dropck1, dropped1) = DropCk::new_pair();
        let (dropck2, dropped2) = DropCk::new_pair();
        let mut world = World::default();
        world.spawn(dropck1).insert(dropck2);
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 0);
        drop(world);
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn insert_overwrite_drop_sparse() {
        let (dropck1, dropped1) = DropCk::new_pair();
        let (dropck2, dropped2) = DropCk::new_pair();
        let mut world = World::default();

        world
            .spawn(DropCkSparse(dropck1))
            .insert(DropCkSparse(dropck2));
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 0);
        drop(world);
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn clear_entities() {
        let mut world = World::default();

        world.insert_resource(A(0));
        world.spawn(A(1));
        world.spawn(SparseStored(1));

        let mut q1 = world.query::<&A>();
        let mut q2 = world.query::<&SparseStored>();

        assert_eq!(q1.iter(&world).len(), 1);
        assert_eq!(q2.iter(&world).len(), 1);
        assert_eq!(world.entities().len(), 2);

        world.clear_entities();

        assert_eq!(
            q1.iter(&world).len(),
            0,
            "world should not contain table components"
        );
        assert_eq!(
            q2.iter(&world).len(),
            0,
            "world should not contain sparse set components"
        );
        assert_eq!(
            world.entities().len(),
            0,
            "world should not have any entities"
        );
        assert_eq!(
            world.resource::<A>().0,
            0,
            "world should still contain resources"
        );
    }

    #[test]
    fn test_is_archetypal_size_hints() {
        let mut world = World::default();
        macro_rules! query_min_size {
            ($query:ty, $filter:ty) => {
                world
                    .query_filtered::<$query, $filter>()
                    .iter(&world)
                    .size_hint()
                    .0
            };
        }

        world.spawn((A(1), B(1), C));
        world.spawn((A(1), C));
        world.spawn((A(1), B(1)));
        world.spawn((B(1), C));
        world.spawn(A(1));
        world.spawn(C);
        assert_eq!(2, query_min_size![(), (With<A>, Without<B>)]);
        assert_eq!(3, query_min_size![&B, Or<(With<A>, With<C>)>]);
        assert_eq!(1, query_min_size![&B, (With<A>, With<C>)]);
        assert_eq!(1, query_min_size![(&A, &B), With<C>]);
        assert_eq!(4, query_min_size![&A, ()], "Simple Archetypal");
        assert_eq!(4, query_min_size![Ref<A>, ()]);
        // All the following should set minimum size to 0, as it's impossible to predict
        // how many entities the filters will trim.
        assert_eq!(0, query_min_size![(), Added<A>], "Simple Added");
        assert_eq!(0, query_min_size![(), Changed<A>], "Simple Changed");
        assert_eq!(0, query_min_size![(&A, &B), Changed<A>]);
        assert_eq!(0, query_min_size![&A, (Changed<A>, With<B>)]);
        assert_eq!(0, query_min_size![(&A, &B), Or<(Changed<A>, Changed<B>)>]);
    }

    #[test]
    fn insert_or_spawn_batch() {
        let mut world = World::default();
        let e0 = world.spawn(A(0)).id();
        let e1 = Entity::from_raw(1);

        let values = vec![(e0, (B(0), C)), (e1, (B(1), C))];

        #[expect(
            deprecated,
            reason = "This needs to be supported for now, and therefore still needs the test."
        )]
        world.insert_or_spawn_batch(values).unwrap();

        assert_eq!(
            world.get::<A>(e0),
            Some(&A(0)),
            "existing component was preserved"
        );
        assert_eq!(
            world.get::<B>(e0),
            Some(&B(0)),
            "pre-existing entity received correct B component"
        );
        assert_eq!(
            world.get::<B>(e1),
            Some(&B(1)),
            "new entity was spawned and received correct B component"
        );
        assert_eq!(
            world.get::<C>(e0),
            Some(&C),
            "pre-existing entity received C component"
        );
        assert_eq!(
            world.get::<C>(e1),
            Some(&C),
            "new entity was spawned and received C component"
        );
    }

    #[test]
    fn insert_or_spawn_batch_invalid() {
        let mut world = World::default();
        let e0 = world.spawn(A(0)).id();
        let e1 = Entity::from_raw(1);
        let e2 = world.spawn_empty().id();
        let invalid_e2 =
            Entity::from_raw_and_generation(e2.index(), NonZero::<u32>::new(2).unwrap());

        let values = vec![(e0, (B(0), C)), (e1, (B(1), C)), (invalid_e2, (B(2), C))];

        #[expect(
            deprecated,
            reason = "This needs to be supported for now, and therefore still needs the test."
        )]
        let result = world.insert_or_spawn_batch(values);

        assert_eq!(
            result,
            Err(vec![invalid_e2]),
            "e2 failed to be spawned or inserted into"
        );

        assert_eq!(
            world.get::<A>(e0),
            Some(&A(0)),
            "existing component was preserved"
        );
        assert_eq!(
            world.get::<B>(e0),
            Some(&B(0)),
            "pre-existing entity received correct B component"
        );
        assert_eq!(
            world.get::<B>(e1),
            Some(&B(1)),
            "new entity was spawned and received correct B component"
        );
        assert_eq!(
            world.get::<C>(e0),
            Some(&C),
            "pre-existing entity received C component"
        );
        assert_eq!(
            world.get::<C>(e1),
            Some(&C),
            "new entity was spawned and received C component"
        );
    }

    #[test]
    fn insert_batch() {
        let mut world = World::default();
        let e0 = world.spawn(A(0)).id();
        let e1 = world.spawn(B(0)).id();

        let values = vec![(e0, (A(1), B(0))), (e1, (A(0), B(1)))];

        world.insert_batch(values);

        assert_eq!(
            world.get::<A>(e0),
            Some(&A(1)),
            "first entity's A component should have been replaced"
        );
        assert_eq!(
            world.get::<B>(e0),
            Some(&B(0)),
            "first entity should have received B component"
        );
        assert_eq!(
            world.get::<A>(e1),
            Some(&A(0)),
            "second entity should have received A component"
        );
        assert_eq!(
            world.get::<B>(e1),
            Some(&B(1)),
            "second entity's B component should have been replaced"
        );
    }

    #[test]
    fn insert_batch_same_archetype() {
        let mut world = World::default();
        let e0 = world.spawn((A(0), B(0))).id();
        let e1 = world.spawn((A(0), B(0))).id();
        let e2 = world.spawn(B(0)).id();

        let values = vec![(e0, (B(1), C)), (e1, (B(2), C)), (e2, (B(3), C))];

        world.insert_batch(values);
        let mut query = world.query::<(Option<&A>, &B, &C)>();
        let component_values = query.get_many(&world, [e0, e1, e2]).unwrap();

        assert_eq!(
            component_values,
            [(Some(&A(0)), &B(1), &C), (Some(&A(0)), &B(2), &C), (None, &B(3), &C)],
            "all entities should have had their B component replaced, received C component, and had their A component (or lack thereof) unchanged"
        );
    }

    #[test]
    fn insert_batch_if_new() {
        let mut world = World::default();
        let e0 = world.spawn(A(0)).id();
        let e1 = world.spawn(B(0)).id();

        let values = vec![(e0, (A(1), B(0))), (e1, (A(0), B(1)))];

        world.insert_batch_if_new(values);

        assert_eq!(
            world.get::<A>(e0),
            Some(&A(0)),
            "first entity's A component should not have been replaced"
        );
        assert_eq!(
            world.get::<B>(e0),
            Some(&B(0)),
            "first entity should have received B component"
        );
        assert_eq!(
            world.get::<A>(e1),
            Some(&A(0)),
            "second entity should have received A component"
        );
        assert_eq!(
            world.get::<B>(e1),
            Some(&B(0)),
            "second entity's B component should not have been replaced"
        );
    }

    #[test]
    fn try_insert_batch() {
        let mut world = World::default();
        let e0 = world.spawn(A(0)).id();
        let e1 = Entity::from_raw(1);

        let values = vec![(e0, (A(1), B(0))), (e1, (A(0), B(1)))];

        let error = world.try_insert_batch(values).unwrap_err();

        assert_eq!(e1, error.entities[0]);

        assert_eq!(
            world.get::<A>(e0),
            Some(&A(1)),
            "first entity's A component should have been replaced"
        );
        assert_eq!(
            world.get::<B>(e0),
            Some(&B(0)),
            "first entity should have received B component"
        );
    }

    #[test]
    fn try_insert_batch_if_new() {
        let mut world = World::default();
        let e0 = world.spawn(A(0)).id();
        let e1 = Entity::from_raw(1);

        let values = vec![(e0, (A(1), B(0))), (e1, (A(0), B(1)))];

        let error = world.try_insert_batch_if_new(values).unwrap_err();

        assert_eq!(e1, error.entities[0]);

        assert_eq!(
            world.get::<A>(e0),
            Some(&A(0)),
            "first entity's A component should not have been replaced"
        );
        assert_eq!(
            world.get::<B>(e0),
            Some(&B(0)),
            "first entity should have received B component"
        );
    }

    #[test]
    fn required_components() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component)]
        #[require(Z = new_z())]
        struct Y {
            value: String,
        }

        #[derive(Component)]
        struct Z(u32);

        impl Default for Y {
            fn default() -> Self {
                Self {
                    value: "hello".to_string(),
                }
            }
        }

        fn new_z() -> Z {
            Z(7)
        }

        let mut world = World::new();
        let id = world.spawn(X).id();
        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world
            .spawn((
                X,
                Y {
                    value: "foo".to_string(),
                },
            ))
            .id();
        assert_eq!(
            "foo",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the manually provided value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world.spawn((X, Z(8))).id();
        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            8,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the manually provided value"
        );
    }

    #[test]
    fn generic_required_components() {
        #[derive(Component)]
        #[require(Y<usize>)]
        struct X;

        #[derive(Component, Default)]
        struct Y<T> {
            value: T,
        }

        let mut world = World::new();
        let id = world.spawn(X).id();
        assert_eq!(
            0,
            world.entity(id).get::<Y<usize>>().unwrap().value,
            "Y should have the default value"
        );
    }

    #[test]
    fn required_components_spawn_nonexistent_hooks() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Resource)]
        struct A(usize);

        #[derive(Resource)]
        struct I(usize);

        let mut world = World::new();
        world.insert_resource(A(0));
        world.insert_resource(I(0));
        world
            .register_component_hooks::<Y>()
            .on_add(|mut world, _| world.resource_mut::<A>().0 += 1)
            .on_insert(|mut world, _| world.resource_mut::<I>().0 += 1);

        // Spawn entity and ensure Y was added
        assert!(world.spawn(X).contains::<Y>());

        assert_eq!(world.resource::<A>().0, 1);
        assert_eq!(world.resource::<I>().0, 1);
    }

    #[test]
    fn required_components_insert_existing_hooks() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Resource)]
        struct A(usize);

        #[derive(Resource)]
        struct I(usize);

        let mut world = World::new();
        world.insert_resource(A(0));
        world.insert_resource(I(0));
        world
            .register_component_hooks::<Y>()
            .on_add(|mut world, _| world.resource_mut::<A>().0 += 1)
            .on_insert(|mut world, _| world.resource_mut::<I>().0 += 1);

        // Spawn entity and ensure Y was added
        assert!(world.spawn_empty().insert(X).contains::<Y>());

        assert_eq!(world.resource::<A>().0, 1);
        assert_eq!(world.resource::<I>().0, 1);
    }

    #[test]
    fn required_components_take_leaves_required() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();
        let e = world.spawn(X).id();
        let _ = world.entity_mut(e).take::<X>().unwrap();
        assert!(world.entity_mut(e).contains::<Y>());
    }

    #[test]
    fn required_components_retain_keeps_required() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component, Default)]
        struct Z;

        let mut world = World::new();
        let e = world.spawn((X, Z)).id();
        world.entity_mut(e).retain::<X>();
        assert!(world.entity_mut(e).contains::<X>());
        assert!(world.entity_mut(e).contains::<Y>());
        assert!(!world.entity_mut(e).contains::<Z>());
    }

    #[test]
    fn required_components_spawn_then_insert_no_overwrite() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y(usize);

        let mut world = World::new();
        let id = world.spawn((X, Y(10))).id();
        world.entity_mut(id).insert(X);

        assert_eq!(
            10,
            world.entity(id).get::<Y>().unwrap().0,
            "Y should still have the manually provided value"
        );
    }

    #[test]
    fn dynamic_required_components() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();
        let x_id = world.register_component::<X>();

        let mut e = world.spawn_empty();

        // SAFETY: x_id is a valid component id
        bevy_ptr::OwningPtr::make(X, |ptr| unsafe {
            e.insert_by_id(x_id, ptr);
        });

        assert!(e.contains::<Y>());
    }

    #[test]
    fn remove_component_and_its_runtime_required_components() {
        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component, Default)]
        struct Z;

        #[derive(Component)]
        struct V;

        let mut world = World::new();
        world.register_required_components::<X, Y>();
        world.register_required_components::<Y, Z>();

        let e = world.spawn((X, V)).id();
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //check that `remove` works as expected
        world.entity_mut(e).remove::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        world.entity_mut(e).insert(X);
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //remove `X` again and ensure that `Y` and `Z` was removed too
        world.entity_mut(e).remove_with_requires::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(!world.entity(e).contains::<Y>());
        assert!(!world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());
    }

    #[test]
    fn remove_component_and_its_required_components() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        #[require(Z)]
        struct Y;

        #[derive(Component, Default)]
        struct Z;

        #[derive(Component)]
        struct V;

        let mut world = World::new();

        let e = world.spawn((X, V)).id();
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //check that `remove` works as expected
        world.entity_mut(e).remove::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        world.entity_mut(e).insert(X);
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //remove `X` again and ensure that `Y` and `Z` was removed too
        world.entity_mut(e).remove_with_requires::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(!world.entity(e).contains::<Y>());
        assert!(!world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());
    }

    #[test]
    fn remove_bundle_and_his_required_components() {
        #[derive(Component, Default)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component, Default)]
        #[require(W)]
        struct Z;

        #[derive(Component, Default)]
        struct W;

        #[derive(Component)]
        struct V;

        #[derive(Bundle, Default)]
        struct TestBundle {
            x: X,
            z: Z,
        }

        let mut world = World::new();
        let e = world.spawn((TestBundle::default(), V)).id();

        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<W>());
        assert!(world.entity(e).contains::<V>());

        world.entity_mut(e).remove_with_requires::<TestBundle>();
        assert!(!world.entity(e).contains::<X>());
        assert!(!world.entity(e).contains::<Y>());
        assert!(!world.entity(e).contains::<Z>());
        assert!(!world.entity(e).contains::<W>());
        assert!(world.entity(e).contains::<V>());
    }

    #[test]
    fn runtime_required_components() {
        // Same as `required_components` test but with runtime registration

        #[derive(Component)]
        struct X;

        #[derive(Component)]
        struct Y {
            value: String,
        }

        #[derive(Component)]
        struct Z(u32);

        impl Default for Y {
            fn default() -> Self {
                Self {
                    value: "hello".to_string(),
                }
            }
        }

        let mut world = World::new();

        world.register_required_components::<X, Y>();
        world.register_required_components_with::<Y, Z>(|| Z(7));

        let id = world.spawn(X).id();

        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world
            .spawn((
                X,
                Y {
                    value: "foo".to_string(),
                },
            ))
            .id();
        assert_eq!(
            "foo",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the manually provided value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world.spawn((X, Z(8))).id();
        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            8,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the manually provided value"
        );
    }

    #[test]
    fn runtime_required_components_override_1() {
        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component)]
        struct Z(u32);

        let mut world = World::new();

        // - X requires Y with default constructor
        // - Y requires Z with custom constructor
        // - X requires Z with custom constructor (more specific than X -> Y -> Z)
        world.register_required_components::<X, Y>();
        world.register_required_components_with::<Y, Z>(|| Z(5));
        world.register_required_components_with::<X, Z>(|| Z(7));

        let id = world.spawn(X).id();

        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in X"
        );
    }

    #[test]
    fn runtime_required_components_override_2() {
        // Same as `runtime_required_components_override_1` test but with different registration order

        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component)]
        struct Z(u32);

        let mut world = World::new();

        // - X requires Y with default constructor
        // - X requires Z with custom constructor (more specific than X -> Y -> Z)
        // - Y requires Z with custom constructor
        world.register_required_components::<X, Y>();
        world.register_required_components_with::<X, Z>(|| Z(7));
        world.register_required_components_with::<Y, Z>(|| Z(5));

        let id = world.spawn(X).id();

        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in X"
        );
    }

    #[test]
    fn runtime_required_components_propagate_up() {
        // `A` requires `B` directly.
        #[derive(Component)]
        #[require(B)]
        struct A;

        #[derive(Component, Default)]
        struct B;

        #[derive(Component, Default)]
        struct C;

        let mut world = World::new();

        // `B` requires `C` with a runtime registration.
        // `A` should also require `C` because it requires `B`.
        world.register_required_components::<B, C>();

        let id = world.spawn(A).id();

        assert!(world.entity(id).get::<C>().is_some());
    }

    #[test]
    fn runtime_required_components_propagate_up_even_more() {
        #[derive(Component)]
        struct A;

        #[derive(Component, Default)]
        struct B;

        #[derive(Component, Default)]
        struct C;

        #[derive(Component, Default)]
        struct D;

        let mut world = World::new();

        world.register_required_components::<A, B>();
        world.register_required_components::<B, C>();
        world.register_required_components::<C, D>();

        let id = world.spawn(A).id();

        assert!(world.entity(id).get::<D>().is_some());
    }

    #[test]
    fn runtime_required_components_deep_require_does_not_override_shallow_require() {
        #[derive(Component)]
        struct A;
        #[derive(Component, Default)]
        struct B;
        #[derive(Component, Default)]
        struct C;
        #[derive(Component)]
        struct Counter(i32);
        #[derive(Component, Default)]
        struct D;

        let mut world = World::new();

        world.register_required_components::<A, B>();
        world.register_required_components::<B, C>();
        world.register_required_components::<C, D>();
        world.register_required_components_with::<D, Counter>(|| Counter(2));
        // This should replace the require constructor in A since it is
        // shallower.
        world.register_required_components_with::<C, Counter>(|| Counter(1));

        let id = world.spawn(A).id();

        // The "shallower" of the two components is used.
        assert_eq!(world.entity(id).get::<Counter>().unwrap().0, 1);
    }

    #[test]
    fn runtime_required_components_deep_require_does_not_override_shallow_require_deep_subtree_after_shallow(
    ) {
        #[derive(Component)]
        struct A;
        #[derive(Component, Default)]
        struct B;
        #[derive(Component, Default)]
        struct C;
        #[derive(Component, Default)]
        struct D;
        #[derive(Component, Default)]
        struct E;
        #[derive(Component)]
        struct Counter(i32);
        #[derive(Component, Default)]
        struct F;

        let mut world = World::new();

        world.register_required_components::<A, B>();
        world.register_required_components::<B, C>();
        world.register_required_components::<C, D>();
        world.register_required_components::<D, E>();
        world.register_required_components_with::<E, Counter>(|| Counter(1));
        world.register_required_components_with::<F, Counter>(|| Counter(2));
        world.register_required_components::<E, F>();

        let id = world.spawn(A).id();

        // The "shallower" of the two components is used.
        assert_eq!(world.entity(id).get::<Counter>().unwrap().0, 1);
    }

    #[test]
    fn runtime_required_components_existing_archetype() {
        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();

        // Registering required components after the archetype has already been created should panic.
        // This may change in the future.
        world.spawn(X);
        assert!(matches!(
            world.try_register_required_components::<X, Y>(),
            Err(RequiredComponentsError::ArchetypeExists(_))
        ));
    }

    #[test]
    fn runtime_required_components_fail_with_duplicate() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();

        // This should fail: Tried to register Y as a requirement for X, but the requirement already exists.
        assert!(matches!(
            world.try_register_required_components::<X, Y>(),
            Err(RequiredComponentsError::DuplicateRegistration(_, _))
        ));
    }

    #[test]
    fn required_components_inheritance_depth() {
        // Test that inheritance depths are computed correctly for requirements.
        //
        // Requirements with `require` attribute:
        //
        // A -> B -> C
        //   0    1
        //
        // Runtime requirements:
        //
        // X -> A -> B -> C
        //   0    1    2
        //
        // X -> Y -> Z -> B -> C
        //   0    1    2    3

        #[derive(Component, Default)]
        #[require(B)]
        struct A;

        #[derive(Component, Default)]
        #[require(C)]
        struct B;

        #[derive(Component, Default)]
        struct C;

        #[derive(Component, Default)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component, Default)]
        struct Z;

        let mut world = World::new();

        let a = world.register_component::<A>();
        let b = world.register_component::<B>();
        let c = world.register_component::<C>();
        let y = world.register_component::<Y>();
        let z = world.register_component::<Z>();

        world.register_required_components::<X, A>();
        world.register_required_components::<X, Y>();
        world.register_required_components::<Y, Z>();
        world.register_required_components::<Z, B>();

        world.spawn(X);

        let required_a = world.get_required_components::<A>().unwrap();
        let required_b = world.get_required_components::<B>().unwrap();
        let required_c = world.get_required_components::<C>().unwrap();
        let required_x = world.get_required_components::<X>().unwrap();
        let required_y = world.get_required_components::<Y>().unwrap();
        let required_z = world.get_required_components::<Z>().unwrap();

        /// Returns the component IDs and inheritance depths of the required components
        /// in ascending order based on the component ID.
        fn to_vec(required: &RequiredComponents) -> Vec<(ComponentId, u16)> {
            let mut vec = required
                .0
                .iter()
                .map(|(id, component)| (*id, component.inheritance_depth))
                .collect::<Vec<_>>();
            vec.sort_by_key(|(id, _)| *id);
            vec
        }

        // Check that the inheritance depths are correct for each component.
        assert_eq!(to_vec(required_a), vec![(b, 0), (c, 1)]);
        assert_eq!(to_vec(required_b), vec![(c, 0)]);
        assert_eq!(to_vec(required_c), vec![]);
        assert_eq!(
            to_vec(required_x),
            vec![(a, 0), (b, 1), (c, 2), (y, 0), (z, 1)]
        );
        assert_eq!(to_vec(required_y), vec![(b, 1), (c, 2), (z, 0)]);
        assert_eq!(to_vec(required_z), vec![(b, 0), (c, 1)]);
    }

    #[test]
    fn required_components_inheritance_depth_bias() {
        #[derive(Component, PartialEq, Eq, Clone, Copy, Debug)]
        struct MyRequired(bool);

        #[derive(Component, Default)]
        #[require(MyRequired(false))]
        struct MiddleMan;

        #[derive(Component, Default)]
        #[require(MiddleMan)]
        struct ConflictingRequire;

        #[derive(Component, Default)]
        #[require(MyRequired(true))]
        struct MyComponent;

        let mut world = World::new();
        let order_a = world
            .spawn((ConflictingRequire, MyComponent))
            .get::<MyRequired>()
            .cloned();
        let order_b = world
            .spawn((MyComponent, ConflictingRequire))
            .get::<MyRequired>()
            .cloned();

        assert_eq!(order_a, Some(MyRequired(true)));
        assert_eq!(order_b, Some(MyRequired(true)));
    }

    #[test]
    #[should_panic = "Recursive required components detected: A → B → C → B\nhelp: If this is intentional, consider merging the components."]
    fn required_components_recursion_errors() {
        #[derive(Component, Default)]
        #[require(B)]
        struct A;

        #[derive(Component, Default)]
        #[require(C)]
        struct B;

        #[derive(Component, Default)]
        #[require(B)]
        struct C;

        World::new().register_component::<A>();
    }

    #[test]
    #[should_panic = "Recursive required components detected: A → A\nhelp: Remove require(A)."]
    fn required_components_self_errors() {
        #[derive(Component, Default)]
        #[require(A)]
        struct A;

        World::new().register_component::<A>();
    }

    #[derive(Default)]
    struct CaptureMapper(Vec<Entity>);
    impl EntityMapper for CaptureMapper {
        fn get_mapped(&mut self, source: Entity) -> Entity {
            self.0.push(source);
            source
        }

        fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
    }

    #[test]
    fn map_struct_entities() {
        #[derive(Component)]
        #[expect(
            unused,
            reason = "extra fields are used to ensure the derive works properly"
        )]
        struct Foo(usize, #[entities] Entity);

        #[derive(Component)]
        #[expect(
            unused,
            reason = "extra fields are used to ensure the derive works properly"
        )]
        struct Bar {
            #[entities]
            a: Entity,
            b: usize,
            #[entities]
            c: Vec<Entity>,
        }

        let mut world = World::new();
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();
        let e3 = world.spawn_empty().id();

        let mut foo = Foo(1, e1);
        let mut mapper = CaptureMapper::default();
        Component::map_entities(&mut foo, &mut mapper);
        assert_eq!(&mapper.0, &[e1]);

        let mut bar = Bar {
            a: e1,
            b: 1,
            c: vec![e2, e3],
        };
        let mut mapper = CaptureMapper::default();
        Component::map_entities(&mut bar, &mut mapper);
        assert_eq!(&mapper.0, &[e1, e2, e3]);
    }

    #[test]
    fn map_enum_entities() {
        #[derive(Component)]
        #[expect(
            unused,
            reason = "extra fields are used to ensure the derive works properly"
        )]
        enum Foo {
            Bar(usize, #[entities] Entity),
            Baz {
                #[entities]
                a: Entity,
                b: usize,
                #[entities]
                c: Vec<Entity>,
            },
        }

        let mut world = World::new();
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();
        let e3 = world.spawn_empty().id();

        let mut foo = Foo::Bar(1, e1);
        let mut mapper = CaptureMapper::default();
        Component::map_entities(&mut foo, &mut mapper);
        assert_eq!(&mapper.0, &[e1]);

        let mut foo = Foo::Baz {
            a: e1,
            b: 1,
            c: vec![e2, e3],
        };
        let mut mapper = CaptureMapper::default();
        Component::map_entities(&mut foo, &mut mapper);
        assert_eq!(&mapper.0, &[e1, e2, e3]);
    }

    #[expect(
        dead_code,
        reason = "This struct is used as a compilation test to test the derive macros, and as such is intentionally never constructed."
    )]
    #[derive(Component)]
    struct ComponentA(u32);

    #[expect(
        dead_code,
        reason = "This struct is used as a compilation test to test the derive macros, and as such is intentionally never constructed."
    )]
    #[derive(Component)]
    struct ComponentB(u32);

    #[derive(Bundle)]
    struct Simple(ComponentA);

    #[derive(Bundle)]
    struct Tuple(Simple, ComponentB);

    #[derive(Bundle)]
    struct Record {
        field0: Simple,
        field1: ComponentB,
    }

    #[derive(Component)]
    struct MyEntities {
        #[entities]
        entities: Vec<Entity>,
        #[entities]
        another_one: Entity,
        #[entities]
        maybe_entity: Option<Entity>,
        #[expect(
            dead_code,
            reason = "This struct is used as a compilation test to test the derive macros, and as such this field is intentionally never used."
        )]
        something_else: String,
    }

    #[expect(
        dead_code,
        reason = "This struct is used as a compilation test to test the derive macros, and as such is intentionally never constructed."
    )]
    #[derive(Component)]
    struct MyEntitiesTuple(#[entities] Vec<Entity>, #[entities] Entity, usize);
}
