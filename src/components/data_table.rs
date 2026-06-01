use dioxus::prelude::*;

pub const DATA_TABLE_PAGE_SIZE: usize = 100;

/// Reusable data table wrapper with consistent styling.
/// Provides the rounded card container, optional toolbar, sticky header,
/// horizontal + vertical scroll, and built-in pagination for 100+ rows.
#[component]
pub fn DataTable(
    /// Column headers: Vec of (label, extra_class) pairs.
    columns: Vec<(String, String)>,
    /// Optional toolbar element rendered above the table.
    #[props(default)]
    toolbar: Option<Element>,
    /// Total number of rows (used for pagination display).
    /// If provided and > PAGE_SIZE, pagination controls are shown.
    #[props(default)]
    total_rows: Option<usize>,
    /// Current page index (0-based). Caller manages page state.
    #[props(default)]
    current_page: Option<usize>,
    /// Callback when the page changes.
    #[props(default)]
    on_page_change: Option<EventHandler<usize>>,
    /// The table body rows (already sliced to the current page by the caller).
    children: Element,
) -> Element {
    let page = current_page.unwrap_or(0);
    let show_pagination = total_rows.is_some_and(|t| t > DATA_TABLE_PAGE_SIZE);
    let total = total_rows.unwrap_or(0);
    let total_pages = if total == 0 { 1 } else { total.div_ceil(DATA_TABLE_PAGE_SIZE) };
    let has_prev = page > 0;
    let has_next = page + 1 < total_pages;

    let prev_class = if has_prev {
        "px-3.5 py-1.5 rounded-lg text-sm bg-gray-900 text-gray-300 hover:bg-gray-800 border border-gray-800 transition-colors"
    } else {
        "px-3.5 py-1.5 rounded-lg text-sm bg-gray-900 text-gray-600 cursor-not-allowed border border-gray-800"
    };
    let next_class = if has_next {
        "px-3.5 py-1.5 rounded-lg text-sm bg-gray-900 text-gray-300 hover:bg-gray-800 border border-gray-800 transition-colors"
    } else {
        "px-3.5 py-1.5 rounded-lg text-sm bg-gray-900 text-gray-600 cursor-not-allowed border border-gray-800"
    };

    let displayed_rows = if show_pagination {
        let start = page * DATA_TABLE_PAGE_SIZE;
        let end = ((page + 1) * DATA_TABLE_PAGE_SIZE).min(total);
        end - start
    } else {
        total
    };

    rsx! {
        div { class: "bg-gray-900 border border-gray-800 rounded-xl overflow-hidden flex flex-col flex-1 min-h-0",
            if let Some(tb) = toolbar {
                div { class: "relative z-20 bg-gray-900 -mb-px", {tb} }
            }
            div { class: "overflow-auto flex-1 min-h-0",
                table { class: "w-full text-sm text-left",
                    thead { class: "bg-gray-900 sticky top-0 z-10",
                        tr { class: "border-b border-gray-800 bg-gray-900",
                            for (label , extra) in columns.iter() {
                                th { class: "px-5 py-3.5 text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap {extra}",
                                    "{label}"
                                }
                            }
                        }
                    }
                    tbody { {children} }
                }
            }
            if show_pagination {
                div { class: "px-5 py-3 border-t border-gray-800 flex items-center justify-between shrink-0",
                    div { class: "text-xs text-gray-500",
                        "Page {page + 1} of {total_pages} \u{2014} {displayed_rows} of {total} rows"
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "{prev_class}",
                            disabled: !has_prev,
                            onclick: move |_| {
                                if has_prev
                                    && let Some(ref handler) = on_page_change
                                {
                                    handler.call(page - 1);
                                }
                            },
                            "Previous"
                        }
                        button {
                            class: "{next_class}",
                            disabled: !has_next,
                            onclick: move |_| {
                                if has_next
                                    && let Some(ref handler) = on_page_change
                                {
                                    handler.call(page + 1);
                                }
                            },
                            "Next"
                        }
                    }
                }
            }
        }
    }
}
