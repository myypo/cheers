pub struct AttrPlugin {
    key: &'static str,
    pub name: &'static str,
    pub path: &'static str,
}

pub struct ActionPlugin {
    key: &'static str,
    pub name: &'static str,
    pub path: &'static str,
    pub is_backend: bool,
}

static ATTR_PLUGINS: &[AttrPlugin] = &[
    AttrPlugin {
        key: "show",
        name: "Show",
        path: "../plugins/attributes/show",
    },
    AttrPlugin {
        key: "bind",
        name: "Bind",
        path: "../plugins/attributes/bind",
    },
    AttrPlugin {
        key: "class",
        name: "Class",
        path: "../plugins/attributes/class",
    },
    AttrPlugin {
        key: "style",
        name: "Style",
        path: "../plugins/attributes/style",
    },
    AttrPlugin {
        key: "text",
        name: "Text",
        path: "../plugins/attributes/text",
    },
    AttrPlugin {
        key: "on",
        name: "On",
        path: "../plugins/attributes/on",
    },
    AttrPlugin {
        key: "attr",
        name: "Attr",
        path: "../plugins/attributes/attr",
    },
    AttrPlugin {
        key: "computed",
        name: "Computed",
        path: "../plugins/attributes/computed",
    },
    AttrPlugin {
        key: "effect",
        name: "Effect",
        path: "../plugins/attributes/effect",
    },
    AttrPlugin {
        key: "indicator",
        name: "Indicator",
        path: "../plugins/attributes/indicator",
    },
    AttrPlugin {
        key: "json-signals",
        name: "JsonSignals",
        path: "../plugins/attributes/jsonSignals",
    },
    AttrPlugin {
        key: "on-intersect",
        name: "OnIntersect",
        path: "../plugins/attributes/onIntersect",
    },
    AttrPlugin {
        key: "on-interval",
        name: "OnInterval",
        path: "../plugins/attributes/onInterval",
    },
    AttrPlugin {
        key: "on-load",
        name: "OnLoad",
        path: "../plugins/attributes/onLoad",
    },
    AttrPlugin {
        key: "on-signal-patch",
        name: "OnSignalPatch",
        path: "../plugins/attributes/onSignalPatch",
    },
    AttrPlugin {
        key: "ref",
        name: "Ref",
        path: "../plugins/attributes/ref",
    },
    AttrPlugin {
        key: "signals",
        name: "Signals",
        path: "../plugins/attributes/signals",
    },
];

static ACTION_PLUGINS: &[ActionPlugin] = &[
    ActionPlugin {
        key: "setAll",
        name: "SetAll",
        path: "../plugins/actions/setAll",
        is_backend: false,
    },
    ActionPlugin {
        key: "toggleAll",
        name: "ToggleAll",
        path: "../plugins/actions/toggleAll",
        is_backend: false,
    },
    ActionPlugin {
        key: "peek",
        name: "Peek",
        path: "../plugins/actions/peek",
        is_backend: false,
    },
    ActionPlugin {
        key: "delete",
        name: "DELETE",
        path: "../plugins/backend/actions/delete",
        is_backend: true,
    },
    ActionPlugin {
        key: "get",
        name: "GET",
        path: "../plugins/backend/actions/get",
        is_backend: true,
    },
    ActionPlugin {
        key: "patch",
        name: "PATCH",
        path: "../plugins/backend/actions/patch",
        is_backend: true,
    },
    ActionPlugin {
        key: "post",
        name: "POST",
        path: "../plugins/backend/actions/post",
        is_backend: true,
    },
    ActionPlugin {
        key: "put",
        name: "PUT",
        path: "../plugins/backend/actions/put",
        is_backend: true,
    },
];

pub struct AttrPlugins;

impl AttrPlugins {
    pub fn get(&self, key: &str) -> Option<&AttrPlugin> {
        ATTR_PLUGINS.iter().find(|p| p.key == key)
    }
}

pub struct ActionPlugins;

impl ActionPlugins {
    pub fn get(&self, key: &str) -> Option<&ActionPlugin> {
        ACTION_PLUGINS.iter().find(|p| p.key == key)
    }
}
