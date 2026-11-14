use std::ops::Range;

use crate::{
    context::DatastarSource,
    reference::Signal,
    render::{
        Buffer, Render, push_datastar_source_to_html_attribute,
        push_js_single_quoted_string_to_html_attribute,
    },
    signal_path::{is_bare_signal_path_segment, parse_signal_path},
};

/// Writes the body of the JS object literal behind `!signals(...)` and `!computed(...)`.
///
/// Each entry is first written as its own nested object (`_a:{b:1},_a:{c:2}`). A JS object
/// literal keeps only the last of repeated keys, so when two entries share a root,
/// [`close`](Self::close) rewrites the body into one merged tree (`_a:{b:1,c:2}`).
#[doc(hidden)]
#[derive(Debug)]
pub struct SignalObject {
    start: usize,
    segments: Vec<Range<usize>>,
    entries: Vec<Entry>,
    open_entry: Option<OpenEntry>,
    needs_rewrite: bool,
}

#[derive(Debug)]
struct Entry {
    /// Indices into [`SignalObject::segments`]. Empty when the signal path had no segments.
    segments: Range<usize>,
    value: Range<usize>,
}

#[derive(Debug)]
struct OpenEntry {
    segments: Range<usize>,
    value_start: usize,
}

impl SignalObject {
    #[doc(hidden)]
    /// Used by the `html!` and `attribute!` macros right after writing the opening `{` of a
    /// signal object. Not part of the stable public API.
    pub fn open(buffer: &mut Buffer<DatastarSource>) -> Self {
        Self {
            start: buffer.dangerously_get_string().len(),
            segments: Vec::new(),
            entries: Vec::new(),
            open_entry: None,
            needs_rewrite: false,
        }
    }

    #[doc(hidden)]
    /// Used by the `html!` and `attribute!` macros when expanding `!signals(...)`.
    /// Not part of the stable public API.
    pub fn assign<T: Render<DatastarSource>>(
        &mut self,
        signal: &Signal<T>,
        value: T,
        buffer: &mut Buffer<DatastarSource>,
    ) {
        self.open_entry(signal.__path(), buffer);
        value.render_to(buffer);
        self.close_entry(buffer);
    }

    #[doc(hidden)]
    /// Used by the `html!` and `attribute!` macros when expanding `!computed(...)`, before
    /// writing the computed body. Not part of the stable public API.
    pub fn computed_open<T>(&mut self, signal: &Signal<T>, buffer: &mut Buffer<DatastarSource>) {
        self.open_entry(signal.__path(), buffer);
        // XSS SAFETY: statically assigning a JS function - the execution is intentional.
        buffer.dangerously_get_string().push_str("()=>");
    }

    #[doc(hidden)]
    /// Used by the `html!` and `attribute!` macros when expanding `!computed(...)`, after
    /// writing the computed body. Not part of the stable public API.
    pub fn computed_close(&mut self, buffer: &mut Buffer<DatastarSource>) {
        self.close_entry(buffer);
    }

    #[doc(hidden)]
    /// Used by the `html!` and `attribute!` macros right before writing the closing `}` of a
    /// signal object. Not part of the stable public API.
    pub fn close(self, buffer: &mut Buffer<DatastarSource>) {
        if !self.needs_rewrite {
            return;
        }

        let dst = buffer.dangerously_get_string();
        let written = dst.split_off(self.start);
        let entries: Vec<usize> = (0..self.entries.len())
            .filter(|&entry| !self.entries[entry].segments.is_empty())
            .collect();
        self.write_merged(&written, &entries, 0, dst);
    }

    fn open_entry(&mut self, path: &str, buffer: &mut Buffer<DatastarSource>) {
        debug_assert!(
            self.open_entry.is_none(),
            "signal object entry opened twice"
        );

        let dst = buffer.dangerously_get_string();
        if !self.entries.is_empty() {
            dst.push(',');
        }

        let segments_start = self.segments.len();
        let mut path_segments = parse_signal_path(path).into_iter();
        if let Some(root) = path_segments.next() {
            let root = self.push_key(dst, &root);
            let root_text = &dst[root.clone()];
            if self.entries.iter().any(|entry| {
                self.segments[entry.segments.clone()]
                    .first()
                    .is_some_and(|other| &dst[other.clone()] == root_text)
            }) {
                self.needs_rewrite = true;
            }

            for segment in path_segments {
                // XSS SAFETY: static object syntax around a key escaped by `push_key`.
                dst.push_str(":{");
                self.push_key(dst, &segment);
            }
            dst.push(':');
        } else {
            self.needs_rewrite = true;
        }

        self.open_entry = Some(OpenEntry {
            segments: segments_start..self.segments.len(),
            value_start: dst.len(),
        });
    }

    fn close_entry(&mut self, buffer: &mut Buffer<DatastarSource>) {
        let Some(OpenEntry {
            segments,
            value_start,
        }) = self.open_entry.take()
        else {
            debug_assert!(false, "signal object entry closed without being opened");
            return;
        };

        let dst = buffer.dangerously_get_string();
        let value = value_start..dst.len();
        for _ in 1..segments.len() {
            // XSS SAFETY: statically closing the nested objects opened in `open_entry`.
            dst.push('}');
        }

        self.entries.push(Entry { segments, value });
    }

    fn push_key(&mut self, dst: &mut String, segment: &str) -> Range<usize> {
        let start = dst.len();
        // XSS SAFETY: signal paths are framework-generated Datastar paths. We emit each object
        // key as either a bare identifier or a quoted JS string literal so arbitrary segments
        // remain valid when embedded inside a double-quoted HTML attribute value.
        if is_bare_signal_path_segment(segment) {
            push_datastar_source_to_html_attribute(dst, segment);
        } else {
            push_js_single_quoted_string_to_html_attribute(dst, segment);
        }
        let key = start..dst.len();
        self.segments.push(key.clone());
        key
    }

    fn key<'a>(&self, written: &'a str, entry: usize, depth: usize) -> &'a str {
        let segment = &self.segments[self.entries[entry].segments.start + depth];
        &written[segment.start - self.start..segment.end - self.start]
    }

    fn depth(&self, entry: usize) -> usize {
        self.entries[entry].segments.len()
    }

    fn path(&self, written: &str, entry: usize) -> String {
        (0..self.depth(entry))
            .map(|depth| self.key(written, entry, depth))
            .collect::<Vec<_>>()
            .join(".")
    }

    fn write_merged(&self, written: &str, entries: &[usize], depth: usize, dst: &mut String) {
        let mut grouped = vec![false; entries.len()];
        let mut first = true;

        for i in 0..entries.len() {
            if grouped[i] {
                continue;
            }

            let key = self.key(written, entries[i], depth);
            let mut group = Vec::new();
            for (j, &entry) in entries.iter().enumerate().skip(i) {
                if !grouped[j] && self.key(written, entry, depth) == key {
                    grouped[j] = true;
                    group.push(entry);
                }
            }

            if !first {
                dst.push(',');
            }
            first = false;

            // XSS SAFETY: keys and values are copied verbatim from text this object already
            // wrote into the attribute, only regrouped under shared keys.
            dst.push_str(key);
            dst.push(':');

            let leaf = group
                .iter()
                .rev()
                .copied()
                .find(|&entry| self.depth(entry) == depth + 1);
            if let Some(leaf) = leaf {
                debug_assert!(
                    group.len() == 1,
                    "`{}` is assigned together with itself or one of its nested signals",
                    self.path(written, leaf),
                );
                let value = &self.entries[leaf].value;
                dst.push_str(&written[value.start - self.start..value.end - self.start]);
            } else {
                dst.push('{');
                self.write_merged(written, &group, depth + 1, dst);
                dst.push('}');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(f: impl FnOnce(&mut SignalObject, &mut Buffer<DatastarSource>)) -> String {
        let mut buffer = Buffer::<DatastarSource>::new();
        let mut object = SignalObject::open(&mut buffer);
        f(&mut object, &mut buffer);
        object.close(&mut buffer);
        buffer.rendered().into_inner()
    }

    fn signal<T>(path: &str) -> Signal<T> {
        Signal::__string(path.to_owned())
    }

    #[test]
    fn string_value() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("user['name']"), "Nick", buffer);
        });
        assert_eq!(rendered, "user:{name:'Nick'}");
    }

    #[test]
    fn number_value() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("user['age']"), -42.0, buffer);
        });
        assert_eq!(rendered, "user:{age:-42.0}");
    }

    #[test]
    fn unsafe_segment() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("project['user.123']['name']"), "Nick", buffer);
        });
        assert_eq!(rendered, "project:{'user.123':{name:'Nick'}}");
    }

    #[test]
    fn distinct_roots_stay_as_written() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("a['x']"), 1, buffer);
            object.assign(&signal("b"), 2, buffer);
        });
        assert_eq!(rendered, "a:{x:1},b:2");
    }

    #[test]
    fn shared_prefixes_merge_at_every_depth() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("a['x']['p']"), 1, buffer);
            object.assign(&signal("b"), 2, buffer);
            object.assign(&signal("a['y']"), 3, buffer);
            object.assign(&signal("a['x']['q']"), 4, buffer);
        });
        assert_eq!(rendered, "a:{x:{p:1,q:4},y:3},b:2");
    }

    #[test]
    fn computed_entries_merge() {
        let rendered = render(|object, buffer| {
            object.computed_open(&signal::<()>("a['x']"), buffer);
            buffer.dangerously_get_string().push('1');
            object.computed_close(buffer);
            object.computed_open(&signal::<()>("a['y']"), buffer);
            buffer.dangerously_get_string().push('2');
            object.computed_close(buffer);
        });
        assert_eq!(rendered, "a:{x:()=>1,y:()=>2}");
    }

    #[test]
    fn keeps_text_written_before_the_object() {
        let mut buffer = Buffer::<DatastarSource>::new();
        buffer.dangerously_get_string().push('{');
        let mut object = SignalObject::open(&mut buffer);
        object.assign(&signal("a['x']"), 1, &mut buffer);
        object.assign(&signal("a['y']"), 2, &mut buffer);
        object.close(&mut buffer);
        assert_eq!(buffer.rendered().into_inner(), "{a:{x:1,y:2}");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(
        expected = "`a.x` is assigned together with itself or one of its nested signals"
    )]
    fn rejects_a_signal_next_to_its_nested_signal() {
        render(|object, buffer| {
            object.assign(&signal("a['x']['y']"), 1, buffer);
            object.assign(&signal("a['x']"), 2, buffer);
        });
    }
}
