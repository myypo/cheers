use std::ops::Range;

use crate::{
    context::DatastarSource,
    reference::Signal,
    render::{
        Buffer, Render, push_datastar_source_to_html_attribute,
        push_js_single_quoted_string_to_html_attribute,
    },
    signal_path::parse_signal_path,
};

/// Writes the body of the JS object literal behind `!signals(...)` and `!computed(...)`.
///
/// Each entry is first written as its own nested object (`_a:{b:1},_a:{c:2}`). A JS object
/// literal keeps only the last of repeated keys, so when two entries share a root,
/// [`close`](Self::close) rewrites the body into one merged tree (`_a:{b:1,c:2}`).
///
/// When one entry's path ends where another's continues (`a.x` and `a.x.y`), the later entry
/// wins, like the last-write-wins merge of `PatchSignals`.
#[doc(hidden)]
#[derive(Debug)]
pub struct SignalObject {
    start: usize,
    /// Buffer length after the last entry; nothing else may be written between entries.
    end: usize,
    segments: Vec<Range<usize>>,
    entries: Vec<Entry>,
    open_entry: Option<OpenEntry>,
    shares_root: bool,
}

#[derive(Debug)]
struct Entry {
    /// Indices into [`SignalObject::segments`], never empty.
    segments: Range<usize>,
    value: Range<usize>,
}

#[derive(Debug)]
struct OpenEntry {
    start: usize,
    segments: Range<usize>,
    value_start: usize,
}

impl SignalObject {
    #[doc(hidden)]
    /// Used by the `html!` and `attribute!` macros right after writing the opening `{` of a
    /// signal object. Not part of the stable public API.
    pub fn open(buffer: &mut Buffer<DatastarSource>) -> Self {
        let start = buffer.dangerously_get_string().len();
        Self {
            start,
            end: start,
            segments: Vec::new(),
            entries: Vec::new(),
            open_entry: None,
            shares_root: false,
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
        let dst = buffer.dangerously_get_string();
        debug_assert!(
            self.open_entry.is_none(),
            "signal object closed inside an entry"
        );
        debug_assert_eq!(
            dst.len(),
            self.end,
            "text was written into a signal object outside its entries"
        );

        if !self.shares_root {
            return;
        }

        let written = dst.split_off(self.start);
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        self.write_merged(&written, &mut order, 0, dst);
    }

    fn open_entry(&mut self, path: &str, buffer: &mut Buffer<DatastarSource>) {
        let dst = buffer.dangerously_get_string();
        debug_assert!(
            self.open_entry.is_none(),
            "signal object entry opened twice"
        );
        debug_assert_eq!(
            dst.len(),
            self.end,
            "text was written into a signal object outside its entries"
        );

        let start = dst.len();
        if !self.entries.is_empty() {
            dst.push(',');
        }

        let segments_start = self.segments.len();
        let mut path_segments = parse_signal_path(path).into_iter();
        if let Some(root) = path_segments.next() {
            let root = self.push_key(dst, &root);
            let root_text = &dst[root];
            self.shares_root |= self.entries.iter().any(|entry| {
                let other = &self.segments[entry.segments.start];
                &dst[other.clone()] == root_text
            });

            for segment in path_segments {
                // XSS SAFETY: static object syntax around a key escaped by `push_key`.
                dst.push_str(":{");
                self.push_key(dst, &segment);
            }
            dst.push(':');
        }

        self.open_entry = Some(OpenEntry {
            start,
            segments: segments_start..self.segments.len(),
            value_start: dst.len(),
        });
    }

    fn close_entry(&mut self, buffer: &mut Buffer<DatastarSource>) {
        let dst = buffer.dangerously_get_string();
        let Some(OpenEntry {
            start,
            segments,
            value_start,
        }) = self.open_entry.take()
        else {
            debug_assert!(false, "signal object entry closed without being opened");
            return;
        };

        if segments.is_empty() {
            dst.truncate(start);
        } else {
            let value = value_start..dst.len();
            for _ in 1..segments.len() {
                // XSS SAFETY: statically closing the nested objects opened in `open_entry`.
                dst.push('}');
            }
            self.entries.push(Entry { segments, value });
        }

        self.end = dst.len();
    }

    fn push_key(&mut self, dst: &mut String, segment: &str) -> Range<usize> {
        let start = dst.len();
        // XSS SAFETY: a key is either bare (see `is_bare_key`) or a quoted JS string literal, both
        // escaped for embedding inside a double-quoted HTML attribute value.
        if is_bare_key(segment) {
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

    /// Writes `entries`, which all share their first `depth` segments, grouped by the segment at
    /// `depth` in first-seen order.
    fn write_merged(&self, written: &str, entries: &mut [usize], depth: usize, dst: &mut String) {
        let mut group_start = 0;
        while group_start < entries.len() {
            let key = self.key(written, entries[group_start], depth);
            let mut group_end = group_start + 1;
            let mut next = group_end;
            while next < entries.len() {
                if self.key(written, entries[next], depth) == key {
                    entries[group_end..=next].rotate_right(1);
                    group_end += 1;
                }
                next += 1;
            }
            let group = &mut entries[group_start..group_end];

            if group_start > 0 {
                dst.push(',');
            }
            // XSS SAFETY: keys and values are copied verbatim from text this object already
            // wrote into the attribute, only regrouped under shared keys.
            dst.push_str(key);
            dst.push(':');

            let last_leaf = group
                .iter()
                .rposition(|&entry| self.depth(entry) == depth + 1);
            match last_leaf {
                Some(leaf) if leaf + 1 == group.len() => {
                    let value = &self.entries[group[leaf]].value;
                    dst.push_str(&written[value.start - self.start..value.end - self.start]);
                }
                nested_after => {
                    let nested = nested_after.map_or(0, |leaf| leaf + 1);
                    dst.push('{');
                    self.write_merged(written, &mut group[nested..], depth + 1, dst);
                    dst.push('}');
                }
            }

            group_start = group_end;
        }
    }
}

/// Whether `segment` can be written as an unquoted object key that JS reads back as the same
/// string: an identifier, or an integer without leading zeros that a double holds exactly.
fn is_bare_key(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if first.is_ascii_digit() {
        return segment.len() <= 15
            && (segment == "0" || first != '0')
            && chars.all(|ch| ch.is_ascii_digit());
    }

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
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
    fn later_assignment_of_the_same_signal_wins() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("a['x']"), 1, buffer);
            object.assign(&signal("a['y']"), 2, buffer);
            object.assign(&signal("a['x']"), 3, buffer);
        });
        assert_eq!(rendered, "a:{x:3,y:2}");
    }

    #[test]
    fn signal_assigned_after_its_nested_signals_replaces_them() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("a['x']['y']"), 1, buffer);
            object.assign(&signal("a['z']"), 2, buffer);
            object.assign(&signal("a['x']"), 3, buffer);
        });
        assert_eq!(rendered, "a:{x:3,z:2}");
    }

    #[test]
    fn nested_signals_assigned_after_their_parent_replace_it() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("a['x']['y']"), 1, buffer);
            object.assign(&signal("a['x']"), 2, buffer);
            object.assign(&signal("a['x']['z']"), 3, buffer);
            object.assign(&signal("a['x']['w']"), 4, buffer);
        });
        assert_eq!(rendered, "a:{x:{z:3,w:4}}");
    }

    #[test]
    fn drops_entries_with_an_empty_path() {
        let rendered = render(|object, buffer| {
            object.assign(&signal(""), 1, buffer);
            object.assign(&signal("a"), 2, buffer);
            object.assign(&signal(""), 3, buffer);
            object.assign(&signal("b"), 4, buffer);
        });
        assert_eq!(rendered, "a:2,b:4");
    }

    #[test]
    fn quotes_keys_that_are_not_js_identifiers() {
        let rendered = render(|object, buffer| {
            object.assign(&signal("_row['550e8400-e29b-41d4']['open']"), true, buffer);
            object.assign(&signal("_row['2fa']['open']"), false, buffer);
            object.assign(&signal("_row['007']['open']"), true, buffer);
            object.assign(&signal("_row['1_000']['open']"), true, buffer);
            object.assign(&signal("_row['1234567890123456']['open']"), true, buffer);
            object.assign(&signal("_row['42']['open']"), true, buffer);
            object.assign(&signal("my-root"), 1, buffer);
        });
        assert_eq!(
            rendered,
            "_row:{'550e8400-e29b-41d4':{open:true},'2fa':{open:false},'007':{open:true},\
             '1_000':{open:true},'1234567890123456':{open:true},42:{open:true}},'my-root':1"
        );
    }
}
