use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagNode {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub note_count: usize,
    pub children: Vec<TagNode>,
}

#[derive(Default)]
struct Builder {
    name: String,
    notes: HashSet<usize>,
    children: BTreeMap<String, Builder>,
}

pub fn build_tag_tree(tag_sets: &[Vec<String>]) -> Vec<TagNode> {
    let mut roots: BTreeMap<String, Builder> = BTreeMap::new();
    for (note_index, tags) in tag_sets.iter().enumerate() {
        let mut seen_paths = HashSet::new();
        for tag in tags {
            let mut path = String::new();
            let mut level = &mut roots;
            for segment in tag.split('/') {
                if segment.is_empty() {
                    continue;
                }
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(segment);
                let entry = level
                    .entry(segment.to_string())
                    .or_insert_with(|| Builder {
                        name: segment.to_string(),
                        ..Default::default()
                    });
                if seen_paths.insert(path.clone()) {
                    entry.notes.insert(note_index);
                }
                level = &mut entry.children;
            }
        }
    }
    fn materialize(builders: BTreeMap<String, Builder>, prefix: &str, depth: usize) -> Vec<TagNode> {
        builders
            .into_iter()
            .map(|(name, builder)| {
                let path = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}/{name}")
                };
                TagNode {
                    note_count: builder.notes.len(),
                    children: materialize(builder.children, &path, depth + 1),
                    path,
                    name,
                    depth,
                }
            })
            .collect()
    }
    materialize(roots, "", 0)
}

pub fn tag_in_subtree(tag: &str, prefix: &str) -> bool {
    let tag = tag.to_lowercase();
    let prefix = prefix.to_lowercase();
    tag == prefix || tag.starts_with(&format!("{prefix}/"))
}
