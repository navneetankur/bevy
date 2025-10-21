use super::*;

/// SAFETY: access of `&T` is a subset of `&mut T`
unsafe impl<'__w, T: Component<Mutability = Mutable>> QueryData for &'__w mut T {
    const IS_READ_ONLY: bool = false;
    type ReadOnly = &'__w T;
    type Item<'w, 's> = &'w mut T;

    fn shrink<'wlong: 'wshort, 'wshort, 's>(
        item: Self::Item<'wlong, 's>,
    ) -> Self::Item<'wshort, 's> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w, 's>(
        state: &'s Self::State,
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w, 's> {
        <Mut<'w, T> as QueryData>::fetch(state, fetch, entity, table_row).into_inner()
    }
}
// SAFETY: access of `Ref<T>` is a subset of `Mut<T>`
unsafe impl<'__w, T: Component<Mutability = Mutable>> QueryData for Mut<'__w, T> {
    const IS_READ_ONLY: bool = false;
    type ReadOnly = Ref<'__w, T>;
    type Item<'w, 's> = Mut<'w, T>;

    // Forwarded to `&mut T`
    fn shrink<'wlong: 'wshort, 'wshort, 's>(
        item: Self::Item<'wlong, 's>,
    ) -> Self::Item<'wshort, 's> {
        item
    }

    #[inline(always)]
    // Forwarded to `&mut T`
    unsafe fn fetch<'w, 's>(
        _state: &'s Self::State,
        // Rust complains about lifetime bounds not matching the trait if I directly use `WriteFetch<'w, T>` right here.
        // But it complains nowhere else in the entire trait implementation.
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w, 's> {
        fetch.components.extract(
            |table| {
                // SAFETY: set_table was previously called
                let (table_components, added_ticks, changed_ticks, callers) =
                    unsafe { table.debug_checked_unwrap() };

                // SAFETY: The caller ensures `table_row` is in range.
                let component = unsafe { table_components.get(table_row.index()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let added = unsafe { added_ticks.get(table_row.index()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let changed = unsafe { changed_ticks.get(table_row.index()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let caller = callers.map(|callers| unsafe { callers.get(table_row.index()) });

                Mut {
                    value: component.deref_mut(),
                    ticks: TicksMut {
                        added: added.deref_mut(),
                        changed: changed.deref_mut(),
                        this_run: fetch.this_run,
                        last_run: fetch.last_run,
                    },
                    changed_by: caller.map(|caller| caller.deref_mut()),
                }
            },
            |sparse_set| {
                // SAFETY: The caller ensures `entity` is in range and has the component.
                let (component, ticks, caller) = unsafe {
                    sparse_set
                        .debug_checked_unwrap()
                        .get_with_ticks(entity)
                        .debug_checked_unwrap()
                };

                Mut {
                    value: component.assert_unique().deref_mut(),
                    ticks: TicksMut::from_tick_cells(ticks, fetch.last_run, fetch.this_run),
                    changed_by: caller.map(|caller| caller.deref_mut()),
                }
            },
        )
    }
}
