/// Splits markdown into chunks along heading boundaries.
///
/// The docs served by this server are all markdown with a meaningful heading
/// structure, so headings give us natural, semantically coherent chunk borders
/// for free - no heuristic sentence splitter needed.
///
/// Fenced code blocks are tracked so that a `#` comment inside a bash/rust
/// snippet is never mistaken for a heading, and so that an oversized section is
/// never split in the middle of a fence.



pub struct RawChunk {
    /// Breadcrumb of the headings this chunk sits under,
    /// e.g. "MCP Middleware Guide > Creating a Tool Call > Step 2".
    pub heading_path: String,
    pub text: String,
}

struct Section {
    heading_path: String,
    lines: Vec<String>,
}

fn heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();

    if !trimmed.starts_with('#') {
        return None;
    }

    let level = trimmed.chars().take_while(|c| *c == '#').count();

    if level == 0 || level > 6 {
        return None;
    }

    // A real ATX heading has a space after the hashes.
    match trimmed.chars().nth(level) {
        Some(' ') => Some(level),
        _ => None,
    }
}

fn heading_text(line: &str) -> String {
    line.trim_start()
        .trim_start_matches('#')
        .trim()
        .trim_end_matches('#')
        .trim()
        .to_string()
}

fn is_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Splits the document into one section per heading.
fn split_into_sections(content: &str) -> Vec<Section> {
    let mut result: Vec<Section> = Vec::new();

    // heading_stack[i] is the heading text at level i+1
    let mut heading_stack: Vec<String> = Vec::new();

    let mut current = Section {
        heading_path: String::new(),
        lines: Vec::new(),
    };

    let mut inside_fence = false;

    for line in content.lines() {
        if is_fence(line) {
            inside_fence = !inside_fence;
            current.lines.push(line.to_string());
            continue;
        }

        if inside_fence {
            current.lines.push(line.to_string());
            continue;
        }

        let level = heading_level(line);

        let Some(level) = level else {
            current.lines.push(line.to_string());
            continue;
        };

        // A new heading closes the previous section.
        if !current.lines.is_empty() || !current.heading_path.is_empty() {
            result.push(current);
        }

        heading_stack.truncate(level - 1);

        while heading_stack.len() < level - 1 {
            heading_stack.push(String::new());
        }

        heading_stack.push(heading_text(line));

        let heading_path = heading_stack
            .iter()
            .filter(|it| !it.is_empty())
            .map(|it| it.as_str())
            .collect::<Vec<_>>()
            .join(" > ");

        current = Section {
            heading_path,
            lines: Vec::new(),
        };
    }

    if !current.lines.is_empty() || !current.heading_path.is_empty() {
        result.push(current);
    }

    result
}

/// Splits an oversized section into several chunks along blank lines, never
/// cutting inside a fenced code block.
fn split_oversized(heading_path: &str, body: &str, max_chunk_chars: usize) -> Vec<RawChunk> {
    let mut result = Vec::new();

    let mut buffer: Vec<String> = Vec::new();
    let mut buffer_len = 0usize;
    let mut inside_fence = false;

    let mut flush = |buffer: &mut Vec<String>, buffer_len: &mut usize| {
        if *buffer_len == 0 {
            buffer.clear();
            return;
        }

        let text = buffer.join("\n").trim().to_string();

        if !text.is_empty() {
            result.push(RawChunk {
                heading_path: heading_path.to_string(),
                text,
            });
        }

        buffer.clear();
        *buffer_len = 0;
    };

    for line in body.lines() {
        if is_fence(line) {
            inside_fence = !inside_fence;
        }

        let is_break_point = !inside_fence && line.trim().is_empty();

        if is_break_point && buffer_len >= max_chunk_chars {
            flush(&mut buffer, &mut buffer_len);
            continue;
        }

        buffer_len += line.len() + 1;
        buffer.push(line.to_string());
    }

    flush(&mut buffer, &mut buffer_len);

    result
}

/// Merges chunks that are too small to carry meaning on their own into the
/// next chunk sharing the same document.
fn merge_tiny(chunks: Vec<RawChunk>, min_chunk_chars: usize) -> Vec<RawChunk> {
    let mut result: Vec<RawChunk> = Vec::with_capacity(chunks.len());

    let mut pending: Option<RawChunk> = None;

    for mut chunk in chunks {
        if let Some(prev) = pending.take() {
            chunk.text = format!("{}\n\n{}", prev.text, chunk.text);
            chunk.heading_path = prev.heading_path;
        }

        if chunk.text.len() < min_chunk_chars {
            pending = Some(chunk);
            continue;
        }

        result.push(chunk);
    }

    // Whatever is left over is still worth keeping - append it to the last
    // chunk if there is one, otherwise emit it as is.
    if let Some(leftover) = pending {
        match result.last_mut() {
            Some(last) => {
                last.text.push_str("\n\n");
                last.text.push_str(&leftover.text);
            }
            None => result.push(leftover),
        }
    }

    result
}

pub fn chunk_markdown(content: &str, max_chunk_chars: usize, min_chunk_chars: usize) -> Vec<RawChunk> {
    let sections = split_into_sections(content);

    let mut chunks = Vec::new();

    for section in sections {
        let body = section.lines.join("\n");
        let body = body.trim();

        if body.is_empty() && section.heading_path.is_empty() {
            continue;
        }

        if body.len() <= max_chunk_chars {
            chunks.push(RawChunk {
                heading_path: section.heading_path,
                text: body.to_string(),
            });
            continue;
        }

        chunks.extend(split_oversized(&section.heading_path, body, max_chunk_chars));
    }

    merge_tiny(chunks, min_chunk_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_inside_code_fence_is_not_a_heading() {
        let content = "# Title\n\nSome intro text that is long enough to matter for the chunker to keep it around as its own chunk, and then some.\n\n```bash\n# this is a comment, not a heading\necho hello\n```\n";

        let chunks = chunk_markdown(content, 2400, 220);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].heading_path, "Title");
        assert!(chunks[0].text.contains("not a heading"));
    }

    #[test]
    fn heading_path_is_a_breadcrumb() {
        let body = "This paragraph is deliberately long enough to clear the MIN_CHUNK_CHARS threshold, so that the tiny-chunk merge step leaves it alone and it survives as a chunk of its own rather than being folded into whichever chunk happens to follow it in the document.";

        let content = format!(
            "# Guide\n\n{}\n\n## Section\n\n{}\n\n### Step 1\n\n{}\n",
            body, body, body
        );

        let chunks = chunk_markdown(&content, 2400, 220);

        let paths: Vec<&str> = chunks.iter().map(|it| it.heading_path.as_str()).collect();

        assert!(paths.contains(&"Guide"), "got {:?}", paths);
        assert!(paths.contains(&"Guide > Section"), "got {:?}", paths);
        assert!(
            paths.contains(&"Guide > Section > Step 1"),
            "got {:?}",
            paths
        );
    }
}
