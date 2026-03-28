#![expect(missing_docs)]

pub mod elements {
    macro_rules! elements {
        {
            $(
                $(#[$meta:meta])*
                $name:ident $(
                    {
                        $(
                            $(#[$attr_meta:meta])*
                            $attr:ident
                        )*
                    }
                )?
            )*
        } => {
            $(
                $(#[$meta])*
                #[expect(
                    non_camel_case_types,
                    reason = "camel case types will be interpreted as components"
                )]
                #[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::marker::Copy)]
                pub struct $name;

                $(
                    #[allow(non_upper_case_globals)]
                    impl $name {
                        $(
                            $(#[$attr_meta])*
                            pub const $attr: $crate::validation::Attribute = $crate::validation::Attribute;
                        )*
                    }
                )?

                impl $crate::validation::Element for $name {
                    type Kind = $crate::validation::Xml;
                }

                impl $crate::validation::attributes::SvgGlobalAttributes for $name {}
            )*
        }
    }

    elements! {
    a {
            href

            target


            r#type

            download

            ping

            rel

            hreflang

            referrerpolicy
        }

        g

        defs

        svg {
            width

            height

            x

            y

            viewBox

            preserveAspectRatio
        }

        symbol {
            width

            height

            x

            y

            viewBox

            preserveAspectRatio

            refX

            refY
        }

        r#use {
            href

            x

            y

            width

            height

        }

        switch

        desc

        metadata

        title

        circle {
            cx

            cy

            r

            pathLength
        }

        ellipse {
            cx

            cy

            rx

            ry

            pathLength
        }

        line {
            x1

            y1

            x2

            y2

            pathLength
        }

        polygon {
            points

            pathLength
        }

        polyline {
            points

            pathLength
        }

        rect {
            x

            y

            width

            height

            rx

            ry

            pathLength
        }

        path {
            d

            pathLength
        }

        text {
            x

            y

            dx

            dy

            rotate

            lengthAdjust

            textLength
        }

        textPath {
            href

            lengthAdjust

            method

            startOffset

            spacing

            side

            textLength

            path

        }

        tspan {
            x

            y

            dx

            dy

            rotate

            lengthAdjust

            textLength
        }

        linearGradient {
            x1

            y1

            x2

            y2

            gradientUnits

            gradientTransform

            spreadMethod

            href

        }

        radialGradient {
            cx

            cy

            r

            fx

            fy

            fr

            gradientUnits

            gradientTransform

            spreadMethod

            href

        }

        stop {
            offset
        }

        clipPath {
            clipPathUnits
        }

        mask {
            x

            y

            width

            height

            maskUnits

            maskContentUnits
        }

        marker {
            markerWidth

            markerHeight

            markerUnits

            orient

            refX

            refY

            viewBox

            preserveAspectRatio
        }

        pattern {
            x

            y

            width

            height

            patternUnits

            patternContentUnits

            patternTransform

            href

            viewBox

            preserveAspectRatio

        }

        filter {
            x

            y

            width

            height

            filterUnits

            primitiveUnits
        }

        feBlend {
            r#in

            in2

            mode

            x

            y

            width

            height

            result
        }

        feColorMatrix {
            r#in

            r#type

            values

            x

            y

            width

            height

            result
        }

        feComponentTransfer {
            r#in

            x

            y

            width

            height

            result
        }

        feComposite {
            r#in

            in2

            operator

            k1

            k2

            k3

            k4

            x

            y

            width

            height

            result
        }

        feConvolveMatrix {
            r#in

            order

            kernelMatrix

            divisor

            bias

            targetX

            targetY

            edgeMode

            kernelUnitLength

            preserveAlpha

            x

            y

            width

            height

            result
        }

        feDiffuseLighting {
            r#in

            surfaceScale

            diffuseConstant

            kernelUnitLength

            x

            y

            width

            height

            result
        }

        feDisplacementMap {
            r#in

            in2

            scale

            xChannelSelector

            yChannelSelector

            x

            y

            width

            height

            result
        }

        feDistantLight {
            azimuth

            elevation
        }

        feDropShadow {
            r#in

            dx

            dy

            stdDeviation

            x

            y

            width

            height

            result
        }

        feFlood {
            x

            y

            width

            height

            result
        }

        feFuncA {
            r#type

            tableValues

            slope

            intercept

            amplitude

            exponent

            offset
        }

        feFuncB {
            r#type

            tableValues

            slope

            intercept

            amplitude

            exponent

            offset
        }

        feFuncG {
            r#type

            tableValues

            slope

            intercept

            amplitude

            exponent

            offset
        }

        feFuncR {
            r#type

            tableValues

            slope

            intercept

            amplitude

            exponent

            offset
        }

        feGaussianBlur {
            r#in

            stdDeviation

            edgeMode

            x

            y

            width

            height

            result
        }

        feImage {
            href

            preserveAspectRatio

            crossorigin

            x

            y

            width

            height

            result

        }

        feMerge {
            x

            y

            width

            height

            result
        }

        feMergeNode {
            r#in
        }

        feMorphology {
            r#in

            operator

            radius

            x

            y

            width

            height

            result
        }

        feOffset {
            r#in

            dx

            dy

            x

            y

            width

            height

            result
        }

        fePointLight {
            x

            y

            z
        }

        feSpecularLighting {
            r#in

            surfaceScale

            specularConstant

            specularExponent

            kernelUnitLength

            x

            y

            width

            height

            result
        }

        feSpotLight {
            x

            y

            z

            pointsAtX

            pointsAtY

            pointsAtZ

            specularExponent

            limitingConeAngle
        }

        feTile {
            r#in

            x

            y

            width

            height

            result
        }

        feTurbulence {
            baseFrequency

            numOctaves

            seed

            stitchTiles

            r#type

            x

            y

            width

            height

            result
        }

        image {
            href

            x

            y

            width

            height

            preserveAspectRatio

            crossorigin

            decoding

        }

        foreignObject {
            x

            y

            width

            height
        }

        animate {
            attributeName

            values

            from

            to

            by

            begin

            dur

            end

            min

            max

            restart

            repeatCount

            repeatDur

            fill

            calcMode

            keyTimes

            keySplines

            additive

            accumulate

            href
        }

        animateMotion {
            values

            from

            to

            by

            begin

            dur

            end

            min

            max

            restart

            repeatCount

            repeatDur

            fill

            calcMode

            keyTimes

            keySplines

            additive

            accumulate

            keyPoints

            path

            rotate

            origin

            href
        }

        animateTransform {
            attributeName

            values

            from

            to

            by

            begin

            dur

            end

            min

            max

            restart

            repeatCount

            repeatDur

            fill

            calcMode

            keyTimes

            keySplines

            additive

            accumulate

            r#type

            href
        }

        mpath {
            href

        }

        set {
            attributeName

            to

            begin

            dur

            end

            min

            max

            restart

            repeatCount

            repeatDur

            fill

            href
        }

        script {
            r#type

            href

            crossorigin

        }

        style {
            r#type

            media

            title
        }

        view {
            viewBox

            preserveAspectRatio
        }    }
}
