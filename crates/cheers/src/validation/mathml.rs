#![expect(missing_docs)]

use crate::validation::{Attribute, Element};

#[expect(non_upper_case_globals, clippy::doc_markdown)]
/// Global MathML attributes.
pub trait MathMlGlobalAttributes: Element {
    const autofocus: Attribute = Attribute;

    const class: Attribute = Attribute;

    const dir: Attribute = Attribute;

    const displaystyle: Attribute = Attribute;

    const id: Attribute = Attribute;

    const mathbackground: Attribute = Attribute;

    const mathcolor: Attribute = Attribute;

    const mathsize: Attribute = Attribute;

    const nonce: Attribute = Attribute;

    const scriptlevel: Attribute = Attribute;

    const style: Attribute = Attribute;

    const tabindex: Attribute = Attribute;
}

pub mod elements {
    use crate::validation::define_validation_elements;

    define_validation_elements! {
        kind = crate::validation::Xml,
        globals = super::MathMlGlobalAttributes,
        {
        math {
            display
        }

        annotation {
            encoding
        }

        annotation_xml {
            encoding
        }

        menclose {
            notation
        }

        merror

        mfrac {
            linethickness
        }

        mi {
            mathvariant
        }

        mmultiscripts {
            mathvariant
        }

        mn

        mo {
            accent
            fence
            form
            largeop
            lspace
            maxsize
            minsize
            movablelimits
            rspace
            separator
            stretchy
            symmetric
        }

        mover {
            accent
        }

        mpadded {
            depth
            height
            lspace
            voffset
            width
        }

        mphantom

        mprescripts

        mroot

        mrow

        ms

        mspace {
            depth
            height
            width
        }

        msqrt

        mstyle

        msub

        msubsup

        msup

        mtable {
            align
            columnalign
            columnlines
            columnspacing
            frame
            framespacing
            rowalign
            rowlines
            rowspacing
            width
        }

        mtd {
            columnspan
            rowspan
            columnalign
            rowalign
        }

        mtext

        mtr {
            columnalign
            rowalign
        }

        munder {
            accentunder
        }

        munderover {
            accent
            accentunder
        }

        semantics
        }
    }
}
