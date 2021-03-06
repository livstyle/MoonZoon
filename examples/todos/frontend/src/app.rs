use zoon::*;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use ulid::Ulid;
use im::Vector;

mod els;

const STORAGE_KEY: &str = "todos-zoon";

type TodoId = Ulid;

blocks!{
    append_blocks![els]

    // ------ Route ------

    #[route]
    #[derive(Copy, Clone)]
    enum Route {
        #[route("active")]
        Active,
        #[route("completed")]
        Completed,
        #[route()]
        Root,
        Unknown,
    }

    #[cache]
    fn route() -> Route {
        url().map(Route::from)
    }

    #[update]
    fn set_route(route: Route) {
        url().set(Url::from(route))
    }

    // ------ Filters ------

    #[derive(Copy, Clone, Eq, PartialEq, EnumIter)]
    enum Filter {
        All,
        Active,
        Completed,
    }

    #[s_var]
    fn filters() -> Vec<Filter> {
        Filter::iter().collect()
    }

    #[cache]
    fn selected_filter() -> Filter {
        match route().inner() {
            Route::Active => Filter::Active,
            Route::Completed => Filter::Completed,
            _ => Filter::All,
        }
    }

    // ------ SelectedTodo ------

    #[s_var]
    fn selected_todo() -> Option<Var<Todo>> {
        None
    }

    #[update]
    fn select_todo(todo: Option<Var<Todo>>) {
        selected_todo().set(todo)
    }

    #[s_var]
    fn selected_todo_title() -> Option<String> {
        let todo = selected_todo().inner()?;
        let title = todo.map(|todo| todo.title.clone());
        Some(title)
    }

    #[update]
    fn set_selected_todo_title(title: String) {
        selected_todo_title().set(title)
    }

    #[update]
    fn save_selected_todo() {
        let title = selected_todo_title().map_mut(Option::take);
        let todo = selected_todo().map_mut(Option::take);
        todo.update_mut(move |todo| todo.title = title);
    }

    // ------ Todos ------

    #[derive(Deserialize, Serialize)]
    struct Todo {
        id: TodoId,
        title: String,
        completed: bool,
    }

    #[s_var]
    fn todo_update_handler() -> VarUpdateHandler<Todo> {
        VarUpdateHandler::new(|_| todos().mark_updated())
    }

    #[s_var]
    fn new_todo_title() -> String {
        String::new()
    }

    #[update]
    fn set_new_todo_title(title: String) {
        new_todo_title().set(title);
    }

    #[update]
    fn add_todo() {
        let title = new_todo_title().map(String::trim);
        if title.is_empty() {
            return;
        }
        new_todo_title().update_mut(String::clear);

        todos().update_mut(|todos| {
            let todo = new_var_c(Todo {
                id: TodoId::new(),
                title,
                completed: false,
            });
            todos.push_front(todo);
        })
    }

    #[update]
    fn remove_todo(todo: Var<Todo>) {
        if Some(todo) == selected_todo().inner() {
            selected_todo().set(None);
        }
        todos().update_mut(|todos| {
            let position = todos.iter_vars().position(|t| t == todo).unwrap();
            todos.remove(position);
        });
    }

    #[update]
    fn toggle_todo(todo: Var<Todo>) {
        todo.update_mut(|todo| todo.checked = !todo.checked);
    }

    // -- all --

    #[s_var]
    fn todos() -> Vector<VarC<Todo>> {
        LocalStorage::get(STORAGE_KEY).unwrap_or_default()
    }

    #[subscription]
    fn store_todos() {
        todos().use_ref(|todos| LocalStorage::insert(STORAGE_KEY, todos));
    }

    #[update]
    fn check_or_uncheck_all(checked: bool) {
        stop!{
            if are_all_completed().inner() {
                todos().use_ref(|todos| todos.iter().for_each(toggle_todo));
            } else {
                active_todos().use_ref(|todos| todos.iter().for_each(toggle_todo));
            }
        }
    }

    #[cache]
    fn todos_count() -> usize {
        todos().map(Vector::len)
    }

    #[cache]
    fn todos_exist() -> bool {
        todos_count().inner() != 0
    }

    // -- completed --

    #[cache]
    fn completed_todos() -> Vector<VarC<Todo>> {
        let mut todos = todos().inner();
        todos.retain(|todo| todo.map(|todo| todo.completed))
        todos
    }

    #[update]
    fn remove_completed() {
        stop!{
            completed_todos().use_ref(|todos| todos.iter().for_each(remove_todo));
        }
    }

    #[cache]
    fn completed_count() -> usize {
        completed_todos().map(Vector::len)
    }

    #[cache]
    fn completed_exist() -> bool {
        completed_count().inner() != 0
    }

    #[cache]
    fn are_all_completed() -> bool {
        todos_count().inner() == completed_count().inner()
    }

    // -- active --

    #[cache]
    fn active_todos() -> Vector<VarC<Todo>> {
        let mut todos = todos().inner();
        todos.retain(|todo| todo.map(|todo| !todo.completed));
        todos
    }

    #[cache]
    fn active_count() -> usize {
        active_todos().map(Vector::len)
    }

    // -- filtered --

    #[cache]
    fn filtered_todos() -> Cache<Vector<VarC<Todo>>> {
        match selected_filter().inner() {
            Filter::All => todos().to_cache(),
            Filter::Active => active_todos(),
            Filter::Completed => completed_todos(),
        }
    }

}
