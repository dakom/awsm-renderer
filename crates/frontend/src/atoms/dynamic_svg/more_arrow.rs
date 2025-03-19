use crate::prelude::*;
use dominator::svg;

pub struct MoreArrow {}

impl MoreArrow {
    pub fn render(hover_signal: impl Signal<Item = bool> + 'static) -> Dom {
        static CLASS: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("width", "0.68763rem")
                .style("height", "0.56256rem")
                .style("transform", "rotate(0deg)")
            }
        });

        svg!("svg", {
            .class(&*CLASS)
            .attrs!{
                "viewBox": "0 0 11 9",
                "fill": "none",
                "xmlns": "http://www.w3.org/2000/svg",
            }
            .child(
                svg!("path", {
                    .attrs!{
                        "id": "Vector",
                        "fill-rule": "evenodd",
                        "clip-rule": "evenodd",
                        "d": "M0.146631 8.14536C0.100144 8.19185 0.0632674 8.24704 0.0381083 8.30778C0.0129493 8.36852 0 8.43362 0 8.49936C0 8.56511 0.0129493 8.63021 0.0381083 8.69095C0.0632674 8.75169 0.100144 8.80688 0.146631 8.85336C0.193119 8.89985 0.248309 8.93673 0.309048 8.96189C0.369788 8.98705 0.434888 9 0.500632 9C0.566375 9 0.631476 8.98705 0.692215 8.96189C0.752954 8.93673 0.808144 8.89985 0.854632 8.85336L4.85463 4.85336C4.90119 4.80692 4.93814 4.75174 4.96334 4.691C4.98855 4.63025 5.00152 4.56513 5.00152 4.49936C5.00152 4.4336 4.98855 4.36848 4.96334 4.30773C4.93814 4.24699 4.90119 4.19181 4.85463 4.14536L0.854632 0.145365C0.808144 0.0988771 0.752954 0.062001 0.692215 0.0368419C0.631476 0.0116828 0.566375 -0.00126648 0.500632 -0.00126648C0.434888 -0.00126648 0.369788 0.0116828 0.309048 0.0368419C0.248309 0.062001 0.193119 0.0988771 0.146631 0.145365C0.100144 0.191853 0.0632674 0.247042 0.0381083 0.307782C0.0129493 0.368521 -4.8983e-10 0.433621 0 0.499365C4.8983e-10 0.565109 0.0129493 0.630209 0.0381083 0.690948C0.0632674 0.751688 0.100144 0.806877 0.146631 0.853365L3.79363 4.49936L0.146631 8.14536ZM6.14663 8.14536C6.05274 8.23925 6 8.36659 6 8.49936C6 8.63214 6.05274 8.75948 6.14663 8.85336C6.24052 8.94725 6.36786 9 6.50063 9C6.63341 9 6.76074 8.94725 6.85463 8.85336L10.8546 4.85336C10.9012 4.80692 10.9381 4.75174 10.9633 4.691C10.9886 4.63025 11.0015 4.56513 11.0015 4.49936C11.0015 4.4336 10.9886 4.36848 10.9633 4.30773C10.9381 4.24699 10.9012 4.19181 10.8546 4.14536L6.85463 0.145365C6.76074 0.0514784 6.63341 -0.00126648 6.50063 -0.00126648C6.36786 -0.00126648 6.24052 0.0514784 6.14663 0.145365C6.05274 0.239252 6 0.366589 6 0.499365C6 0.632141 6.05274 0.759479 6.14663 0.853365L9.79363 4.49936L6.14663 8.14536Z",
                    }

                    .attr_signal("fill", hover_signal.map(|hover| {
                        if !hover {
                            ColorRaw::MidGrey.value()
                        } else {
                            ColorRaw::Darkish.value()
                        }
                    }))
                })
            )
        })
    }
}
