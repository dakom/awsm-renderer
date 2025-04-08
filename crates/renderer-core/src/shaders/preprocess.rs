/// Processes WGSL-like shader code by removing lines or sections
/// based on custom `// #IF <id>` or `// #SECTIONIF <id>` directives.
/// and a provided `retain` function.
///
/// If `#SECTIONIF <id>` is encountered (and retain(id, code) evaluates to false),
/// it will remove the entire section until the matching `#ENDIF` directive
///
/// * `retain(id, full_source)` is a function/closure that determines if `id` is active.
///   If `false`, the line or section is removed from the final output.
///
/// The second parameter passed to `retain` is the full source code, in case that context is needed.
pub fn preprocess_shader<F>(code: &str, retain: F) -> String
where
    F: Fn(&str, &str) -> bool,
{
    /// A small helper: returns Some("myId") if line ends with `// #IF myId`.
    fn parse_if(line: &str) -> Option<&str> {
        // Trim whitespace
        let line = line.trim();
        // We look for something like `// #IF myId`
        // The simplest approach: look for the substring "// #IF "
        // and parse whatever comes after it.
        const PREFIX: &str = "// #IF ";
        if let Some(pos) = line.find(PREFIX) {
            // Make sure there's nothing after that except the ID
            let id_part = &line[pos + PREFIX.len()..];
            // If `id_part` is empty, or if there's trailing comment, handle that how you like
            // For simplicity, return everything up to the end
            Some(id_part.trim())
        } else {
            None
        }
    }

    /// Returns Some("myId") if line ends with `// #SECTIONIF myId`.
    fn parse_section_if(line: &str) -> Option<&str> {
        let line = line.trim();
        const PREFIX: &str = "// #SECTIONIF ";
        if let Some(pos) = line.find(PREFIX) {
            Some(line[pos + PREFIX.len()..].trim())
        } else {
            None
        }
    }

    /// Returns `true` if line is `// #ENDIF`
    fn parse_end_if(line: &str) -> bool {
        let line = line.trim();
        line == "// #ENDIF"
    }

    let lines: Vec<&str> = code.lines().collect();
    let mut output = Vec::with_capacity(lines.len());

    // We'll keep a stack of booleans indicating if the current section is active.
    // If the top is `false`, we ignore lines until we find a matching // #ENDIF
    // (taking nesting into account).
    let mut section_stack = vec![true];

    // We also need to handle the "skip exactly one next line" scenario for #IF false
    // so let's keep a small "skip counter" for the next line if needed.
    let mut skip_next_line = false;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // If we flagged "skip_next_line" from a #IF false, skip *this* line
        // in the final output, but we still parse it in case it has a directive that
        // might affect nesting. That means "skip in output" but still "process directive."
        if skip_next_line {
            skip_next_line = false; // reset
                                    // We do not add it to output, but we do still parse to handle nested sections
            if let Some(id) = parse_section_if(line) {
                // Even though the parent said skip, we must push a new section.
                let is_parent_active = *section_stack.last().unwrap();
                let is_active = is_parent_active && retain(id, code);
                section_stack.push(is_active);
            } else if parse_end_if(line) {
                // close a section
                let _ = section_stack.pop();
            } else if let Some(id) = parse_if(line) {
                // If parent is active, check if we skip the next line again
                let is_parent_active = *section_stack.last().unwrap();
                if is_parent_active && !retain(id, code) {
                    skip_next_line = true; // skip the next line
                }
            }
            i += 1;
            continue;
        }

        // If the current top of the stack is not active, we skip everything
        // except that we still must track #SECTIONIF / #ENDIF for nesting.
        if !*section_stack.last().unwrap() {
            // We skip this line from the output, but parse it for directives:
            if let Some(_id) = parse_section_if(line) {
                // We are already in a false section, so this nested section is also false.
                section_stack.push(false);
            } else if parse_end_if(line) {
                let _ = section_stack.pop();
            } else if let Some(_id) = parse_if(line) {
                // #IF within a false section means skip the next line too
                // but effectively, everything is already being skipped anyway.
                // We can do nothing or set skip_next_line = true. Either is fine,
                // because we won't emit output lines anyway while the top is false.
                // But to keep logic consistent, let's do it so we parse properly:
                skip_next_line = true;
            }
            i += 1;
            continue;
        }

        //
        // If we get here, the top of the stack is active, so we might:
        //  - see a #IF => possibly skip next line
        //  - see a #SECTIONIF => push new section (active or inactive)
        //  - see a #ENDIF => pop
        //  - else => keep the line in output
        //

        // 1) Check if line is `// #SECTIONIF id`
        if let Some(id) = parse_section_if(line) {
            let is_parent_active = *section_stack.last().unwrap();
            let is_active = is_parent_active && retain(id, code);
            section_stack.push(is_active);
            // Don't output the directive line itself
            i += 1;
            continue;
        }

        // 2) Check if line is `// #ENDIF`
        if parse_end_if(line) {
            // pop the stack
            let _ = section_stack.pop();
            // skip the directive line
            i += 1;
            continue;
        }

        // 3) Check if line is `// #IF id`
        if let Some(id) = parse_if(line) {
            // If we don't retain, we skip exactly the next line
            if !retain(id, code) {
                skip_next_line = true;
            }
            // skip the directive line itself
            i += 1;
            continue;
        }

        // If none of the above, this line is kept in the output
        output.push(line);
        i += 1;
    }

    // Finally, join them back with newlines
    output.join("\n")
}

#[cfg(test)]
mod test {
    #[test]
    fn test_preprocess_shader() {
        let shader_code = r#"
            // #IF myId
            @group(0) @binding(0) var<uniform> myUniform1: f32;

            // #IF myId2
            @group(0) @binding(0) var<uniform> myUniform2: f32;

            @group(0) @binding(0) var<uniform> myUniform3: f32;

            void main() {
                // #SECTIONIF myId3
                vec3 color1 = vec3(1.0, 0.0, 0.0);
                // #ENDIF

                // #SECTIONIF myId4
                vec3 color2 = vec3(1.0, 0.0, 0.0);
                // #ENDIF

                vec3 color3 = vec3(1.0, 0.0, 0.0);
            }
        "#;

        let expected_code = r#"

            @group(0) @binding(0) var<uniform> myUniform2: f32;

            @group(0) @binding(0) var<uniform> myUniform3: f32;

            void main() {

                vec3 color2 = vec3(1.0, 0.0, 0.0);

                vec3 color3 = vec3(1.0, 0.0, 0.0);
            }
        "#;

        let retain = |id: &str, _code: &str| -> bool {
            match id {
                "myId" => false,
                "myId2" => true,
                "myId3" => false,
                "myId4" => true,
                _ => true,
            }
        };
        let processed_code = super::preprocess_shader(shader_code, retain);

        assert_eq!(processed_code, expected_code);
    }
}
