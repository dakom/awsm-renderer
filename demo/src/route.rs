use crate::prelude::*;
use std::fmt::{Debug, Display};
use futures_signals::signal::Signal;
use web_sys::Url;


#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    Home,
    NotFound,
}

impl Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.into();
        write!(f, "{}", s)
    }
}

impl Route {
    pub fn current_signal() -> impl Signal<Item = Self> {
        dominator::routing::url()
            .signal_cloned()
            .map(|url| Self::from_url(&url))
    }

    pub fn go_to_url(&self) {
        let s: String = self.into();
        dominator::routing::go_to_url(&s);
    }

    pub fn hard_redirect(&self) {
        let location = web_sys::window().unwrap_ext().location();
        let s: String = self.into();
        location.set_href(&s).unwrap_ext();
    }

    pub fn push_state(&self) {
        let history = web_sys::window().unwrap_ext().history().unwrap_ext();
        let url: String = self.into();
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&url));
    }

    pub fn from_url(url: &str) -> Self {
        let url = Url::new(url).unwrap_ext();
        let paths = url.pathname();
        let paths = paths.split('/').into_iter().skip(1).collect::<Vec<_>>();
        let paths = paths.as_slice();
        //let params_map = url.search_params();

        //let mut params_string = url.search();
        //if params_string.len() > 1 {
            //// if there's more then one char than it's a '?', so remove it
             //params_string = params_string[1..params_string.len()].to_string();
            ////let query = serde_qs::from_str(&params_string).unwrap_ext();
        //}

        match paths {
            [""] => Self::Home,
            _ => Self::NotFound,
        }
    }
}

impl From<Route> for String {
    fn from(route: Route) -> Self {
        (&route).into()
    }
}

impl From<&Route> for String {
    fn from(route: &Route) -> Self {
        match route {
            Route::Home => "/".to_string(),
            Route::NotFound => "404".to_string(),
        }
    }
}