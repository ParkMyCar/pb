//! A Trie Map data structure.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    ffi::OsString,
    fmt::{self, Debug},
    path::Path,
    rc::Rc,
};

use derivative::Derivative;
use smallvec::SmallVec;

/// A prefix trie data structure that supports map-like operations.
///
/// You can store data along each edge, and on leaf nodes.
#[derive(Debug)]
pub struct TrieMap<K: TrieKey, E, L> {
    root: TrieNode<K, E, L>,
}

impl<K: TrieKey, E, L> TrieMap<K, E, L> {
    /// Create a new [`Trie`] with the provided root data.
    pub fn new_with_root(data: E) -> TrieMap<K, E, L> {
        TrieMap {
            root: TrieNode::Edge {
                children: BTreeMap::default(),
                data,
            },
        }
    }

    pub fn from_node(node: TrieNode<K, E, L>) -> Self {
        TrieMap { root: node }
    }

    /// Insert a piece of data at the provided `path`.
    ///
    /// # Errors
    ///
    /// * If a component in the provided path does not exist as an edge.
    pub fn insert(&mut self, path: K, data: L) -> Result<Option<TrieNode<K, E, L>>, anyhow::Error> {
        let mut node = &mut self.root;
        let mut components: SmallVec<[_; 8]> = path.as_components().collect();
        let Some(last_component) = components.pop() else {
            anyhow::bail!("inserting an empty key is not allowed");
        };

        // Walk down the trie to our final location.
        for component in &components {
            match node {
                TrieNode::Leaf { .. } => {
                    return Err(anyhow::anyhow!("non-edge in path: {components:?}"));
                }
                TrieNode::Edge { children, .. } => {
                    node = children
                        .get_mut(component)
                        .ok_or_else(|| anyhow::anyhow!("missing edge in path: {components:?}"))?;
                }
            }
        }

        // Insert the new child.
        match node {
            TrieNode::Leaf { .. } => Err(anyhow::anyhow!("non-edge parent {components:?}")),
            TrieNode::Edge { children, .. } => {
                let prev = children.insert(last_component.clone(), TrieNode::Leaf { data });
                Ok(prev)
            }
        }
    }

    /// Get the node at the provided path.
    pub fn get(&self, path: K) -> Option<&TrieNode<K, E, L>> {
        let mut node = &self.root;
        for component in path.as_components() {
            match node {
                TrieNode::Leaf { .. } => return None,
                TrieNode::Edge { children, .. } => {
                    node = children.get(&component)?;
                }
            }
        }
        Some(node)
    }

    /// Get the leaf node at the provided path, if the path exists and points to a leaf.
    pub fn get_leaf(&self, path: K) -> Option<&L> {
        match self.get(path)? {
            TrieNode::Edge { .. } => None,
            TrieNode::Leaf { data } => Some(data),
        }
    }
}

impl<K: TrieKey, E: Default, L> TrieMap<K, E, L> {
    /// Create a new [`Trie`] with default data for the root node.
    pub fn new() -> TrieMap<K, E, L> {
        TrieMap::new_with_root(Default::default())
    }

    /// Insert a piece of data at the provided `path`, creating the necessary edges.
    ///
    /// The data stored for an Edge node (aka the `E` generic param) must implement [`Default`].
    pub fn insert_leaf(
        &mut self,
        path: K,
        data: L,
    ) -> Result<Option<TrieNode<K, E, L>>, anyhow::Error> {
        let mut node = &mut self.root;
        let mut components: SmallVec<[_; 8]> = path.as_components().collect();
        let Some(last_component) = components.pop() else {
            anyhow::bail!("inserting an empty key is not allowed");
        };

        // Walk down the trie to our final location.
        for component in &components {
            match node {
                TrieNode::Leaf { .. } => {
                    return Err(anyhow::anyhow!("non-edge in path: {components:?}"));
                }
                TrieNode::Edge { children, .. } => {
                    node = children
                        .entry((*component).clone())
                        .or_insert_with(|| TrieNode::Edge {
                            children: BTreeMap::default(),
                            data: E::default(),
                        })
                }
            }
        }

        // Insert the new child.
        match node {
            TrieNode::Leaf { .. } => Err(anyhow::anyhow!("non-edge parent {components:?}")),
            TrieNode::Edge { children, .. } => {
                let prev = children.insert(last_component.clone(), TrieNode::Leaf { data });
                Ok(prev)
            }
        }
    }
}

/// Single node within a [`TrieMap`].
#[derive(Debug)]
pub enum TrieNode<K: TrieKey, E, L> {
    Edge {
        children: BTreeMap<K::Component, TrieNode<K, E, L>>,
        data: E,
    },
    Leaf {
        data: L,
    },
}

impl<K, E, L> Clone for TrieNode<K, E, L>
where
    K: TrieKey + Clone,
    K::Component: Clone,
    E: Clone,
    L: Clone,
{
    fn clone(&self) -> Self {
        match self {
            TrieNode::Edge { children, data } => TrieNode::Edge {
                children: children.clone(),
                data: data.clone(),
            },
            TrieNode::Leaf { data } => TrieNode::Leaf { data: data.clone() },
        }
    }
}

/// A key within a [`TrieMap`].
///
/// A [`TrieMap`] is a prefix trie like data structure, and as such each key must be broken into
/// separate components.
pub trait TrieKey {
    type Component: Ord + Debug + Clone;

    fn as_components(&self) -> impl Iterator<Item = Self::Component>;
}

impl TrieKey for pb_types::InternedPath {
    type Component = pb_types::InternedComponent;

    fn as_components(&self) -> impl Iterator<Item = Self::Component> {
        self.0.iter().copied()
    }
}

impl<K: TrieKey + Clone, E: Clone, L: Clone> TrieMap<K, E, L> {
    /// Return a [`PrettyTrieNode`] which can be pretty printed.
    pub fn pretty<'a, F>(&'a self, fmt_name: F) -> PrettyTrieNode<'a, K, E, L>
    where
        F: for<'w> Fn(&'w mut dyn std::io::Write, &K::Component) -> std::io::Result<()> + 'a,
    {
        PrettyTrieNode {
            name: None,
            node: self.root.clone(),
            fmt_name: Rc::new(fmt_name),
            fmt_edge: None,
            fmt_leaf: None,
        }
    }
}

/// Helper struct for implementing [`ptree`]'s traits.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct PrettyTrieNode<'a, K: TrieKey, E, L> {
    name: Option<K::Component>,
    node: TrieNode<K, E, L>,

    #[derivative(Debug = "ignore")]
    fmt_name:
        Rc<dyn for<'w> Fn(&'w mut dyn std::io::Write, &K::Component) -> std::io::Result<()> + 'a>,
    #[derivative(Debug = "ignore")]
    fmt_edge: Option<Rc<dyn for<'w> Fn(&'w mut dyn std::io::Write, &E) -> fmt::Result>>,
    #[derivative(Debug = "ignore")]
    fmt_leaf: Option<Rc<dyn for<'w> Fn(&'w mut dyn std::io::Write, &L) -> fmt::Result>>,
}

impl<'a, K, E, L> ptree::TreeItem for PrettyTrieNode<'a, K, E, L>
where
    K: TrieKey + Clone,
    E: Clone,
    L: Clone,
{
    type Child = PrettyTrieNode<'a, K, E, L>;

    fn write_self<W: std::io::Write>(
        &self,
        f: &mut W,
        _style: &ptree::Style,
    ) -> std::io::Result<()> {
        // TODO: Also print the data associated with each node.
        if let Some(name) = &self.name {
            match self.node {
                TrieNode::Leaf { .. } => (self.fmt_name)(f, name),
                TrieNode::Edge { .. } => (self.fmt_name)(f, name),
            }
        } else {
            Ok(())
        }
    }

    fn children(&self) -> Cow<[Self::Child]> {
        match &self.node {
            TrieNode::Leaf { .. } => Cow::Owned(vec![]),
            TrieNode::Edge { children, .. } => {
                let children: Vec<_> = children
                    .iter()
                    .map(|(name, node)| PrettyTrieNode {
                        name: Some(name.clone()),
                        node: node.clone(),
                        fmt_name: Rc::clone(&self.fmt_name),
                        fmt_edge: self.fmt_edge.as_ref().map(|f| Rc::clone(f)),
                        fmt_leaf: self.fmt_leaf.as_ref().map(|f| Rc::clone(f)),
                    })
                    .collect();

                Cow::Owned(children)
            }
        }
    }
}

impl<'a, K, E, L> fmt::Display for PrettyTrieNode<'a, K, E, L>
where
    K: TrieKey + Clone,
    E: Clone,
    L: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: This isn't optimal at all, just threw enough code at this to make it work.
        let mut buf = Vec::new();
        ptree::write_tree(self, &mut buf).expect("TODO");
        let buf = String::from_utf8_lossy(&buf[..]);
        write!(f, "{buf}")?;
        Ok(())
    }
}
