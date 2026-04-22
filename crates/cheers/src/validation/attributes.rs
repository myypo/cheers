#![allow(non_upper_case_globals)]
#[cfg(feature = "mathml")]
pub use super::mathml::MathMlGlobalAttributes;
use crate::validation::{Attribute, Element};

pub trait GlobalAttributes: Element {
    /// Used as a guide for creating a keyboard shortcut that activates or
    /// focuses the element.
    const access_key: Attribute = Attribute;

    /// The autocapitalization behavior to use when the text is edited
    /// through non-keyboard methods.
    const autocapitalize: Attribute = Attribute;

    /// Indicates whether the element should be automatically focused when
    /// the page is loaded.
    const autofocus: Attribute = Attribute;

    /// The class of the element.
    const class: Attribute = Attribute;

    /// Whether the element is editable.
    const contenteditable: Attribute = Attribute;

    /// The text directionality of the element.
    const dir: Attribute = Attribute;

    /// Whether the element is draggable.
    const draggable: Attribute = Attribute;

    /// A hint as to what the `enter` key should do.
    const enterkeyhint: Attribute = Attribute;

    /// Whether the element is hidden from view.
    const hidden: Attribute = Attribute;

    /// A unique identifier for the element.
    const id: Attribute = Attribute;

    /// Mark an element and its children as inert, disabling interaction.
    const inert: Attribute = Attribute;

    /// Specifies what kind of input mechanism would be most helpful for
    /// users entering content.
    const inputmode: Attribute = Attribute;

    /// Specify which element this is a custom variant of.
    const is: Attribute = Attribute;

    /// A global identifier for the item.
    const itemid: Attribute = Attribute;

    /// A property that the item has.
    const itemprop: Attribute = Attribute;

    /// A list of additional elements to crawl to find the name-value pairs
    /// of the item.
    const itemref: Attribute = Attribute;

    /// Creates a new item, a group of name-value pairs.
    const itemscope: Attribute = Attribute;

    /// The item types of the item.
    const itemtype: Attribute = Attribute;

    /// The language of the element.
    const lang: Attribute = Attribute;

    /// A cryptographic nonce ("number used once") which can be used by
    /// Content Security Policy to determine whether or not a given
    /// fetch will be allowed to proceed.
    const nonce: Attribute = Attribute;

    /// When specified, the element won't be rendered until it becomes
    /// shown, at which point it will be rendered on top of other
    /// page content.
    const popover: Attribute = Attribute;

    /// The slot the element is inserted in.
    const slot: Attribute = Attribute;

    /// Whether the element is spellchecked or not.
    const spellcheck: Attribute = Attribute;

    /// The CSS styling to apply to the element.
    const style: Attribute = Attribute;

    /// Customize the index of the element for sequential focus navigation.
    const tabindex: Attribute = Attribute;

    /// A text description of the element.
    const title: Attribute = Attribute;

    /// Whether the element is to be translated when the page is localized.
    const translate: Attribute = Attribute;
}

/// [ARIA](https://www.w3.org/TR/wai-aria/) attribute namespace.
#[expect(missing_docs, non_upper_case_globals)]
pub mod aria {
    use super::Attribute;

    /// Marker type for the ARIA namespace.
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy)]
    pub struct Namespace;

    pub const activedescendant: Attribute = Attribute;
    pub const atomic: Attribute = Attribute;
    pub const autocomplete: Attribute = Attribute;
    pub const braillelabel: Attribute = Attribute;
    pub const brailleroledescription: Attribute = Attribute;
    pub const busy: Attribute = Attribute;
    pub const checked: Attribute = Attribute;
    pub const colcount: Attribute = Attribute;
    pub const colindex: Attribute = Attribute;
    pub const colindextext: Attribute = Attribute;
    pub const colspan: Attribute = Attribute;
    pub const controls: Attribute = Attribute;
    pub const current: Attribute = Attribute;
    pub const describedby: Attribute = Attribute;
    pub const description: Attribute = Attribute;
    pub const details: Attribute = Attribute;
    pub const disabled: Attribute = Attribute;
    pub const dropeffect: Attribute = Attribute;
    pub const errormessage: Attribute = Attribute;
    pub const expanded: Attribute = Attribute;
    pub const flowto: Attribute = Attribute;
    pub const grabbed: Attribute = Attribute;
    pub const haspopup: Attribute = Attribute;
    pub const hidden: Attribute = Attribute;
    pub const invalid: Attribute = Attribute;
    pub const keyshortcuts: Attribute = Attribute;
    pub const label: Attribute = Attribute;
    pub const labelledby: Attribute = Attribute;
    pub const level: Attribute = Attribute;
    pub const live: Attribute = Attribute;
    pub const modal: Attribute = Attribute;
    pub const multiline: Attribute = Attribute;
    pub const multiselectable: Attribute = Attribute;
    pub const orientation: Attribute = Attribute;
    pub const owns: Attribute = Attribute;
    pub const placeholder: Attribute = Attribute;
    pub const posinset: Attribute = Attribute;
    pub const pressed: Attribute = Attribute;
    pub const readonly: Attribute = Attribute;
    pub const relevant: Attribute = Attribute;
    pub const required: Attribute = Attribute;
    pub const roledescription: Attribute = Attribute;
    pub const rowcount: Attribute = Attribute;
    pub const rowindex: Attribute = Attribute;
    pub const rowindextext: Attribute = Attribute;
    pub const rowspan: Attribute = Attribute;
    pub const selected: Attribute = Attribute;
    pub const setsize: Attribute = Attribute;
    pub const sort: Attribute = Attribute;
    pub const valuemax: Attribute = Attribute;
    pub const valuemin: Attribute = Attribute;
    pub const valuenow: Attribute = Attribute;
    pub const valuetext: Attribute = Attribute;
}

/// Trait providing the ARIA namespace for elements.
pub trait AriaAttributes: GlobalAttributes {
    /// The ARIA attribute namespace.
    const aria: aria::Namespace = aria::Namespace;
    /// The role attribute.
    const role: Attribute = Attribute;
}

impl<T: GlobalAttributes> AriaAttributes for T {}

/// [Datastar](https://data-star.dev) attribute namespace.
#[expect(missing_docs, non_upper_case_globals)]
pub mod data {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy)]
    pub struct Namespace;

    use crate::validation::Attribute;

    /// Sets the value of any HTML attribute to an expression, and keeps it in
    /// sync.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: String)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     div !attr("title": signal_foo) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const attr: Attribute = Attribute;

    /// Creates a signal (if one doesn't already exist) and sets up two-way data
    /// binding between it and an element's value.
    ///
    /// This means that the value of the element is updated when the signal
    /// changes, and the signal value is updated when the value of the element
    /// changes.
    ///
    /// The `data-bind` attribute can be placed on any HTML element on which
    /// data can be input or choices selected (`input`, `select`,`textarea`
    /// elements, and web components). Event listeners are added for `change`
    /// and `input` events.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: String)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     input !bind(signal_foo);
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    ///
    /// The initial value of the signal is set to the value of the element,
    /// unless a signal has already been defined.
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: String)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     input !bind(signal_foo) value="bar";
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    ///
    /// # Predefined Signal Types
    ///
    /// When you predefine a signal, its **type** is preserved during binding.
    /// Whenever the element's value changes, the signal value is automatically
    /// converted to match the original type.
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: i32)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     div !signals(signal_foo: 0) {
    ///         select !bind(signal_foo) {
    ///             option value="10" { "10" }
    ///         }
    ///     }
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    ///
    /// In the same way, you can assign multiple input values to a single signal
    /// by predefining it as an **array**.
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         scoped_signal!(signal_foo: Vec<String>);
    /// html! {
    ///     div !signals(signal_foo: Vec::<String>::new()) {
    ///         input !bind(signal_foo) type="checkbox" value="bar";
    ///         input !bind(signal_foo) type="checkbox" value="baz";
    ///     }
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    ///
    /// # File Uploads
    ///
    /// Input fields of type `file` will automatically encode file contents in
    /// base64. This means that a form is not required.
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(files: ())]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_files);
    /// html! {
    ///     input type="file" !bind(signal_files) multiple;
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    ///
    /// The resulting signal is in the format `{ name: string, contents: string,
    /// mime: string }[]`.
    pub const bind: Attribute = Attribute;

    /// Adds or removes a class to or from an element based on an expression.
    ///
    /// If the expression evaluates to `true`, the class is added to the
    /// element; otherwise, it is removed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(is_hidden: bool)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_is_hidden);
    /// html! {
    ///     div !class({ "{hidden: " (signal_is_hidden) "}" }) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const class: Attribute = Attribute;

    /// Creates a signal that is computed based on an expression.
    ///
    /// The computed signal is read-only, and its value is automatically updated
    /// when any signals in the expression are updated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: i32)]
    /// # #[signal(bar: i32)]
    /// # #[signal(total: i32)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo, signal_bar, signal_total);
    /// html! {
    ///     div !computed(signal_total: { (signal_foo) " + " (signal_bar) }) {}
    ///     div !text(signal_total) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    ///
    /// Computed signals are useful for memoizing expressions containing other
    /// signals. Their values can be used in other expressions.
    ///
    /// > Computed signal expressions must not be used for performing actions
    /// > (changing other signals, actions, JavaScript functions, etc.). If you
    /// > need to perform an action in response to a signal change, use the
    /// > [`data-effect`](#data-effect) attribute.
    pub const computed: Attribute = Attribute;

    /// Executes an expression on page load and whenever any signals in the
    /// expression change.
    ///
    /// This is useful for performing side effects, such as updating other
    /// signals, making requests to the backend, or manipulating the DOM.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: i32)]
    /// # #[signal(bar: i32)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo, signal_bar);
    /// html! {
    ///     div !effect({ (signal_foo) " = " (signal_bar) " + 1" }) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const effect: Attribute = Attribute;

    /// Tells Datastar to ignore an element and its descendants.
    ///
    /// Datastar walks the entire DOM and applies plugins to each element it
    /// encounters. It's possible to tell Datastar to ignore an element and its
    /// descendants by placing a `data-ignore` attribute on it. This can be
    /// useful for preventing naming conflicts with third-party libraries, or
    /// when you are unable to [escape user
    /// input](/reference/security#escape-user-input).
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// html! {
    ///     div !ignore {
    ///         div {
    ///             "Datastar will not process this element."
    ///         }
    ///     }
    /// };
    /// ```
    pub const ignore: Attribute = Attribute;

    /// Tells the PatchElements watcher to skip processing an element and its
    /// children when morphing elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// html! {
    ///     div !ignore_morph {
    ///         "This element will not be morphed."
    ///     }
    /// };
    /// ```
    ///
    /// > To remove the `data-ignore-morph` attribute from an element, simply
    /// > patch the element with the `data-ignore-morph` attribute removed.
    pub const ignore_morph: Attribute = Attribute;

    /// Creates a signal and sets its value to `true` while a fetch request is
    /// in flight, otherwise `false`.
    ///
    /// The signal can be used to show a loading indicator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         scoped_signal!(signal_fetching: bool);
    /// html! {
    ///     button !on:click("@get('/endpoint')") !indicator(signal_fetching) {}
    ///     div !show(signal_fetching) { "Loading..." }
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const indicator: Attribute = Attribute;

    /// Runs an expression when the attribute is initialized.
    ///
    /// This can happen on page load, when an element is patched into the DOM,
    /// and any time the attribute is modified (via a backend action or
    /// otherwise).
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(count: i32)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_count);
    /// html! {
    ///     div !init({ (signal_count) " = 1" }) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const init: Attribute = Attribute;

    /// Sets the text content of an element to a reactive JSON stringified
    /// version of signals.
    ///
    /// Useful when troubleshooting an issue.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// html! {
    ///     pre !json_signals {}
    /// };
    /// ```
    pub const json_signals: Attribute = Attribute;

    /// Preserves the value of an attribute when morphing DOM elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// html! {
    ///     details open !preserve_attr("open") {
    ///         summary { "Title" }
    ///         "Content"
    ///     }
    /// };
    /// ```
    pub const preserve_attr: Attribute = Attribute;

    /// Creates a new signal that is a reference to the element on which the
    /// data attribute is placed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: ())]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     div !ref(signal_foo) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const r#ref: Attribute = Attribute;

    /// Shows or hides an element based on whether an expression evaluates to
    /// `true` or `false`.
    ///
    /// For anything with custom requirements, use [`data-class`](#data-class)
    /// instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: bool)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     div !show(signal_foo) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const show: Attribute = Attribute;

    /// Patches (adds, updates or removes) one or more signals into the existing
    /// signals.
    ///
    /// Values defined later in the DOM tree override those defined earlier.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: i32)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     div !signals(signal_foo: 1) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const signals: Attribute = Attribute;

    /// Sets the value of inline CSS styles on an element based on an
    /// expression, and keeps them in sync.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(using_red: bool)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_using_red);
    /// html! {
    ///     div !style("background-color": { (signal_using_red) " ? 'red' : 'blue'" }) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const style: Attribute = Attribute;

    /// Binds the text content of an element to an expression.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cheers::prelude::*;
    /// # #[derive(Cheers)]
    /// # #[signal(foo: i32)]
    /// # struct Example {
    /// #     #[id]
    /// #     id: u8,
    /// # }
    /// # impl Render for Example {
    /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
    /// #         signals!(signal_foo);
    /// html! {
    ///     div !text(signal_foo) {}
    ///     // or with a complex expression
    ///     div !text({ "'Value: ' + (" (signal_foo) " * 2)" }) {}
    /// }
    /// #         .render_to(buffer);
    /// #     }
    /// # }
    /// # let _ = Example { id: 0 }.render();
    /// ```
    pub const text: Attribute = Attribute;

    /// Event listener attribute namespace.
    pub mod on {
        #[derive(Debug, Clone, Copy)]
        pub struct Namespace;

        use crate::validation::Attribute;

        // Standard DOM Events

        // Mouse Events
        /// Fired when a pointing device button (e.g., a mouse's primary button)
        /// is pressed and released on a single element.
        pub const click: Attribute = Attribute;
        /// Fired when a pointing device button (e.g., a mouse's primary button)
        /// is clicked twice on a single element.
        pub const dblclick: Attribute = Attribute;
        /// Fired when a non-primary pointing device button is clicked (e.g.,
        /// middle or right mouse button).
        pub const auxclick: Attribute = Attribute;
        /// Fired when a pointing device button is pressed on an element.
        pub const mousedown: Attribute = Attribute;
        /// Fired when a pointing device button is released on an element.
        pub const mouseup: Attribute = Attribute;
        /// Fired when a pointing device (usually a mouse) is moved while over
        /// an element.
        pub const mousemove: Attribute = Attribute;
        /// Fired when a pointing device is moved onto the element to which the
        /// listener is attached or onto one of its children.
        pub const mouseover: Attribute = Attribute;
        /// Fired when a pointing device (usually a mouse) is moved off the
        /// element to which the listener is attached or off one of its
        /// children.
        pub const mouseout: Attribute = Attribute;
        /// Fired when a pointing device (usually a mouse) is moved over the
        /// element that has the listener attached.
        pub const mouseenter: Attribute = Attribute;
        /// Fired when the pointer of a pointing device (usually a mouse) is
        /// moved out of an element that has the listener attached to it.
        pub const mouseleave: Attribute = Attribute;
        /// Fired when the user attempts to open a context menu.
        pub const contextmenu: Attribute = Attribute;

        // Keyboard Events
        /// Fired when a key is pressed.
        pub const keydown: Attribute = Attribute;
        /// Fired when a key is released.
        pub const keyup: Attribute = Attribute;
        /// Fired when a key that produces a character value is pressed down.
        pub const keypress: Attribute = Attribute;

        // Input Events
        /// Fired when the value of an input element is about to be modified.
        pub const beforeinput: Attribute = Attribute;
        /// Fired when an element's value is changed as a direct result of a
        /// user action.
        pub const input: Attribute = Attribute;

        // Composition Events
        /// Fired when text composition begins (e.g., via IME).
        pub const compositionstart: Attribute = Attribute;
        /// Fired when a character is added to a text composition session.
        pub const compositionupdate: Attribute = Attribute;
        /// Fired when text composition ends.
        pub const compositionend: Attribute = Attribute;

        // Form Events
        /// Fired when a form is submitted.
        pub const submit: Attribute = Attribute;
        /// Fired when the value of an input element is changed as a direct
        /// result of a user action.
        pub const change: Attribute = Attribute;
        /// Fired after the form data has been constructed.
        pub const formdata: Attribute = Attribute;
        /// Fired when an element has gained focus.
        pub const focus: Attribute = Attribute;
        /// Fired when an element has lost focus.
        pub const blur: Attribute = Attribute;
        /// Fired when an element has gained focus, after focus.
        pub const focusin: Attribute = Attribute;
        /// Fired when an element has lost focus, after blur.
        pub const focusout: Attribute = Attribute;
        /// Fired when a submittable element has been checked for validity and
        /// doesn't satisfy its constraints.
        pub const invalid: Attribute = Attribute;
        /// Fired when a form is reset.
        pub const reset: Attribute = Attribute;
        /// Fired when some text is selected.
        pub const select: Attribute = Attribute;

        // Drag Events
        /// Fired when an element or text selection is being dragged.
        pub const drag: Attribute = Attribute;
        /// Fired when the user starts dragging an element or text selection.
        pub const dragstart: Attribute = Attribute;
        /// Fired when a drag operation is being ended (by releasing a mouse
        /// button or hitting the escape key).
        pub const dragend: Attribute = Attribute;
        /// Fired when a dragged element or text selection enters a valid drop
        /// target.
        pub const dragenter: Attribute = Attribute;
        /// Fired when a dragged element or text selection leaves a valid drop
        /// target.
        pub const dragleave: Attribute = Attribute;
        /// Fired when an element or text selection is being dragged over a
        /// valid drop target.
        pub const dragover: Attribute = Attribute;
        /// Fired when an element or text selection is dropped on a valid drop
        /// target.
        pub const drop: Attribute = Attribute;

        // Clipboard Events
        /// Fired when the user initiates a copy action through the browser's
        /// user interface.
        pub const copy: Attribute = Attribute;
        /// Fired when the user initiates a cut action through the browser's
        /// user interface.
        pub const cut: Attribute = Attribute;
        /// Fired when the user initiates a paste action through the browser's
        /// user interface.
        pub const paste: Attribute = Attribute;

        // Media Events
        /// Fired when the media has enough data to start playing, after the
        /// play event, but also when recovering from being stalled.
        pub const play: Attribute = Attribute;
        /// Fired when a request to pause play is handled and the activity has
        /// entered its paused state, most commonly occurring when the media's
        /// pause() method is called.
        pub const pause: Attribute = Attribute;
        /// Fired when playback stops when end of the media is reached or
        /// because no further data is available.
        pub const ended: Attribute = Attribute;
        /// Fired when either the volume or the muted attribute has changed.
        pub const volumechange: Attribute = Attribute;
        /// Fired when the time indicated by the currentTime attribute has been
        /// updated.
        pub const timeupdate: Attribute = Attribute;
        /// Fired when the user agent can play the media, but estimates that not
        /// enough data has been loaded to play the media up to its end without
        /// having to stop for further buffering of content.
        pub const canplay: Attribute = Attribute;
        /// Fired when the user agent can play the media, and estimates that
        /// enough data has been loaded to play the media up to its end without
        /// having to stop for further buffering of content.
        pub const canplaythrough: Attribute = Attribute;
        /// Fired when the duration attribute has been updated.
        pub const durationchange: Attribute = Attribute;
        /// Fired when the media has become empty; for example, when the media
        /// has already been loaded (or partially loaded), and the load() method
        /// is called to reload it.
        pub const emptied: Attribute = Attribute;
        /// Fired when the first frame of the media has finished loading.
        pub const loadeddata: Attribute = Attribute;
        /// Fired when the metadata has been loaded.
        pub const loadedmetadata: Attribute = Attribute;
        /// Fired when the browser starts looking for media data.
        pub const loadstart: Attribute = Attribute;
        /// Fired when the media begins to play (either for the first time,
        /// after having been paused, or after ending and then restarting).
        pub const playing: Attribute = Attribute;
        /// Fired periodically as the browser loads a resource.
        pub const progress: Attribute = Attribute;
        /// Fired when the playback rate has changed.
        pub const ratechange: Attribute = Attribute;
        /// Fired when a seek operation completes.
        pub const seeked: Attribute = Attribute; // typos: ignore
        /// Fired when a seek operation begins.
        pub const seeking: Attribute = Attribute;
        /// Fired when the user agent is trying to fetch media data, but data is
        /// unexpectedly not forthcoming.
        pub const stalled: Attribute = Attribute;
        /// Fired when media data loading has been suspended.
        pub const suspend: Attribute = Attribute;
        /// Fired when playback has stopped because of a temporary lack of data.
        pub const waiting: Attribute = Attribute;

        // Touch Events
        /// Fired when one or more touch points are placed on the touch surface.
        pub const touchstart: Attribute = Attribute;
        /// Fired when one or more touch points are moved along the touch
        /// surface.
        pub const touchmove: Attribute = Attribute;
        /// Fired when one or more touch points are removed from the touch
        /// surface.
        pub const touchend: Attribute = Attribute;
        /// Fired when one or more touch points have been disrupted in an
        /// implementation-specific manner.
        pub const touchcancel: Attribute = Attribute;

        // Pointer Events
        /// Fired when a pointer becomes active.
        pub const pointerdown: Attribute = Attribute;
        /// Fired when a pointer is no longer active.
        pub const pointerup: Attribute = Attribute;
        /// Fired when a pointer changes coordinates.
        pub const pointermove: Attribute = Attribute;
        /// Fired when a pointer is moved into an element's hit test boundaries.
        pub const pointerover: Attribute = Attribute;
        /// Fired when a pointer is moved out of the hit test boundaries of an
        /// element.
        pub const pointerout: Attribute = Attribute;
        /// Fired when a pointer is moved into the hit test boundaries of an
        /// element or one of its descendants.
        pub const pointerenter: Attribute = Attribute;
        /// Fired when a pointer is moved out of the hit test boundaries of an
        /// element.
        pub const pointerleave: Attribute = Attribute;
        /// Fired when a pointer event is canceled.
        pub const pointercancel: Attribute = Attribute;
        /// Fired when an element captures a pointer using setPointerCapture().
        pub const gotpointercapture: Attribute = Attribute;
        /// Fired when a captured pointer is released.
        pub const lostpointercapture: Attribute = Attribute;

        // Scroll Events
        /// Fired when the document view or an element has been scrolled.
        pub const scroll: Attribute = Attribute;
        /// Fires when the document view has completed scrolling.
        pub const scrollend: Attribute = Attribute;

        // Wheel Events
        /// Fired when the user rotates a wheel button on a pointing device
        /// (typically a mouse).
        pub const wheel: Attribute = Attribute;

        // Animation Events
        /// Fired when an animation starts.
        pub const animationstart: Attribute = Attribute;
        /// Fired when an animation has completed normally.
        pub const animationend: Attribute = Attribute;
        /// Fired when an animation iteration has completed.
        pub const animationiteration: Attribute = Attribute;
        /// Fired when an animation unexpectedly aborts.
        pub const animationcancel: Attribute = Attribute;

        // Transition Events
        /// Fired when a CSS transition has started transitioning.
        pub const transitionstart: Attribute = Attribute;
        /// Fired when a CSS transition has finished playing.
        pub const transitionend: Attribute = Attribute;
        /// Fired when a CSS transition is created.
        pub const transitionrun: Attribute = Attribute;
        /// Fired when a CSS transition has been cancelled.
        pub const transitioncancel: Attribute = Attribute;

        // Window/Document Events
        /// Fired when the whole page has loaded, including all dependent
        /// resources such as stylesheets, scripts, iframes, and images.
        pub const load: Attribute = Attribute;
        /// Fired when the initial HTML document has been completely parsed,
        /// without waiting for stylesheets, images, and subframes to finish
        /// loading.
        pub const DOMContentLoaded: Attribute = Attribute;
        /// Fired when the document readyState property changes.
        pub const readystatechange: Attribute = Attribute;
        /// Fired when the document or a child resource is being unloaded.
        pub const unload: Attribute = Attribute;
        /// Fired when the window, the document and its resources are about to
        /// be unloaded.
        pub const beforeunload: Attribute = Attribute;
        /// Fired when navigating away from a page.
        pub const pagehide: Attribute = Attribute;
        /// Fired when a page is shown, including from back-forward cache.
        pub const pageshow: Attribute = Attribute;
        /// Fired when the document view has been resized.
        pub const resize: Attribute = Attribute;
        /// Fired when a resource failed to load, or can't be used.
        pub const error: Attribute = Attribute;
        /// Fired when a resource loading is aborted.
        pub const abort: Attribute = Attribute;

        // Navigation/History Events
        /// Fired when the active history entry changes.
        pub const popstate: Attribute = Attribute;
        /// Fired when the URL hash fragment changes.
        pub const hashchange: Attribute = Attribute;

        // Connectivity Events
        /// Fired when the browser gains network connection.
        pub const online: Attribute = Attribute;
        /// Fired when the browser loses network connection.
        pub const offline: Attribute = Attribute;

        // Messaging Events
        /// Fired when a message is received from a postMessage call, Worker, or
        /// other messaging source.
        pub const message: Attribute = Attribute;
        /// Fired when a message cannot be deserialized.
        pub const messageerror: Attribute = Attribute;

        // Storage Events
        /// Fired when localStorage or sessionStorage is modified in another
        /// browsing context.
        pub const storage: Attribute = Attribute;

        // Promise Events
        /// Fired when a Promise is rejected and there is no rejection handler.
        pub const unhandledrejection: Attribute = Attribute;
        /// Fired when a handler is attached to a previously rejected Promise.
        pub const rejectionhandled: Attribute = Attribute;

        // Print Events
        /// Fired before the print dialog is opened.
        pub const beforeprint: Attribute = Attribute;
        /// Fired after the print dialog is closed.
        pub const afterprint: Attribute = Attribute;

        // Language Events
        /// Fired when the user's preferred languages change.
        pub const languagechange: Attribute = Attribute;

        // Toggle Events
        /// Fired when the open/closed state of a `<details>` element is toggled.
        pub const toggle: Attribute = Attribute;

        // Popover Events
        /// Fired on a popover element just before it is shown or hidden.
        pub const beforetoggle: Attribute = Attribute;

        // HTML Element Events
        /// Fired when the nodes in a `<slot>` element change.
        pub const slotchange: Attribute = Attribute;
        /// Fired when a `<dialog>` element is canceled (e.g., via ESC key).
        pub const cancel: Attribute = Attribute;
        /// Fired when a `<dialog>` element is closed.
        pub const close: Attribute = Attribute;

        // Fullscreen Events
        /// Fired when entering or exiting fullscreen mode.
        pub const fullscreenchange: Attribute = Attribute;
        /// Fired when fullscreen mode cannot be enabled.
        pub const fullscreenerror: Attribute = Attribute;

        // Page Visibility Events
        /// Fired when the page visibility state changes (e.g., tab hidden or
        /// shown).
        pub const visibilitychange: Attribute = Attribute;

        // Security Events
        /// Fired when a Content Security Policy is violated.
        pub const securitypolicyviolation: Attribute = Attribute;

        // Selection Events
        /// Fired when the user starts selecting text.
        pub const selectstart: Attribute = Attribute;
        /// Fired when the text selection in a `<textarea>` or `<input>` element has
        /// changed.
        pub const selectionchange: Attribute = Attribute;

        // Custom datastar events
        /// Runs an expression when the element intersects with the viewport.
        ///
        /// # Examples
        ///
        /// ```
        /// # use cheers::prelude::*;
        /// # #[derive(Cheers)]
        /// # #[signal(intersected: bool)]
        /// # struct Example {
        /// #     #[id]
        /// #     id: u8,
        /// # }
        /// # impl Render for Example {
        /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
        /// #         signals!(signal_intersected);
        /// html! {
        ///     div !on:intersect({ (signal_intersected) " = true" }) {}
        /// }
        /// #         .render_to(buffer);
        /// #     }
        /// # }
        /// # let _ = Example { id: 0 }.render();
        /// ```
        pub const intersect: Attribute = Attribute;

        /// Runs an expression at a regular interval.
        ///
        /// # Examples
        ///
        /// ```
        /// # use cheers::prelude::*;
        /// # #[derive(Cheers)]
        /// # #[signal(count: i32)]
        /// # struct Example {
        /// #     #[id]
        /// #     id: u8,
        /// # }
        /// # impl Render for Example {
        /// #     fn render_to(&self, buffer: &mut Buffer<Element>) {
        /// #         signals!(signal_count);
        /// html! {
        ///     div !on:interval({ (signal_count) "++" }) {}
        /// }
        /// #         .render_to(buffer);
        /// #     }
        /// # }
        /// # let _ = Example { id: 0 }.render();
        /// ```
        pub const interval: Attribute = Attribute;

        /// Runs an expression whenever any signals are patched.
        ///
        /// This is useful for tracking changes, updating computed values, or
        /// triggering side effects when data updates.
        ///
        /// # Examples
        ///
        /// ```
        /// # use cheers::prelude::*;
        /// html! {
        ///     div !on:signal_patch("console.log('A signal changed!')") {}
        /// };
        /// ```
        pub const signal_patch: Attribute = Attribute;

        /// Filters which signals to watch when using the
        /// [`data-on-signal-patch`](#data-on-signal-patch) attribute.
        ///
        /// # Examples
        ///
        /// ```
        /// # use cheers::prelude::*;
        /// html! {
        ///     div !on:signal_patch_filter("{include: /^counter$/}") {}
        /// };
        /// ```
        pub const signal_patch_filter: Attribute = Attribute;
    }
}

pub trait DataAttributes: GlobalAttributes {
    const data: data::Namespace = data::Namespace;
}

impl<T: GlobalAttributes> DataAttributes for T {}

/// Attributes for [Open Graph protocol](https://ogp.me/) metadata.
///
/// This trait is implemented only for the
/// [`meta`](crate::validation::elements::meta) element and is intended for
/// tags such as `<meta property="og:title" content="...">`.
///
/// # Examples
///
/// ```
/// # use cheers::prelude::*;
/// let result = html! {
///     head {
///         meta property="og:title" content="Cheers";
///         meta property="og:description" content="Fullstack hypermedia framework";
///     }
/// }
/// .render();
///
/// assert_eq!(
///     result.as_inner(),
///     r#"<head><meta property="og:title" content="Cheers"><meta property="og:description" content="Fullstack hypermedia framework"></head>"#,
/// );
/// ```
///
/// See the Open Graph [Basic Metadata](https://ogp.me/#metadata) section for
/// details and required properties.
pub trait OpenGraphMeta: GlobalAttributes {
    /// The Open Graph property key (for example, `og:title` or `og:image`).
    ///
    /// Use this with [`meta`](crate::validation::elements::meta) elements
    /// alongside [`content`](crate::validation::elements::meta::content).
    const property: Attribute = Attribute;
}

impl OpenGraphMeta for crate::validation::elements::meta {}

#[expect(missing_docs)]
pub trait SvgGlobalAttributes: Element {
    const id: Attribute = Attribute;

    const class: Attribute = Attribute;

    const style: Attribute = Attribute;

    const tabindex: Attribute = Attribute;

    const autofocus: Attribute = Attribute;

    const lang: Attribute = Attribute;

    const xml: Attribute = Attribute;

    const xmlns: Attribute = Attribute;

    const required_extensions: Attribute = Attribute;

    const system_language: Attribute = Attribute;

    const alignment_baseline: Attribute = Attribute;
    const baseline_shift: Attribute = Attribute;
    const clip: Attribute = Attribute;
    const clip_path: Attribute = Attribute;
    const clip_rule: Attribute = Attribute;
    const color: Attribute = Attribute;
    const color_interpolation: Attribute = Attribute;
    const color_interpolation_filters: Attribute = Attribute;
    const cursor: Attribute = Attribute;
    const d: Attribute = Attribute;
    const direction: Attribute = Attribute;
    const display: Attribute = Attribute;
    const dominant_baseline: Attribute = Attribute;
    const enable_background: Attribute = Attribute;
    const fill: Attribute = Attribute;
    const fill_opacity: Attribute = Attribute;
    const fill_rule: Attribute = Attribute;
    const filter: Attribute = Attribute;
    const flood_color: Attribute = Attribute;
    const flood_opacity: Attribute = Attribute;
    const font_family: Attribute = Attribute;
    const font_size: Attribute = Attribute;
    const font_size_adjust: Attribute = Attribute;
    const font_stretch: Attribute = Attribute;
    const font_style: Attribute = Attribute;
    const font_variant: Attribute = Attribute;
    const font_weight: Attribute = Attribute;
    const glyph_orientation_horizontal: Attribute = Attribute;
    const glyph_orientation_vertical: Attribute = Attribute;
    const image_rendering: Attribute = Attribute;
    const kerning: Attribute = Attribute;
    const letter_spacing: Attribute = Attribute;
    const lighting_color: Attribute = Attribute;
    const marker_end: Attribute = Attribute;
    const marker_mid: Attribute = Attribute;
    const marker_start: Attribute = Attribute;
    const mask: Attribute = Attribute;
    const opacity: Attribute = Attribute;
    const overflow: Attribute = Attribute;
    const paint_order: Attribute = Attribute;
    const pointer_events: Attribute = Attribute;
    const shape_rendering: Attribute = Attribute;
    const stop_color: Attribute = Attribute;
    const stop_opacity: Attribute = Attribute;
    const stroke: Attribute = Attribute;
    const stroke_dasharray: Attribute = Attribute;
    const stroke_dashoffset: Attribute = Attribute;
    const stroke_linecap: Attribute = Attribute;
    const stroke_linejoin: Attribute = Attribute;
    const stroke_miterlimit: Attribute = Attribute;
    const stroke_opacity: Attribute = Attribute;
    const stroke_width: Attribute = Attribute;
    const text_anchor: Attribute = Attribute;
    const text_decoration: Attribute = Attribute;
    const text_rendering: Attribute = Attribute;
    const transform: Attribute = Attribute;
    const transform_origin: Attribute = Attribute;
    const unicode_bidi: Attribute = Attribute;
    const vector_effect: Attribute = Attribute;
    const visibility: Attribute = Attribute;
    const word_spacing: Attribute = Attribute;
    const writing_mode: Attribute = Attribute;
}
