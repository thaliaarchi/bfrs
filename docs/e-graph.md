# E-graph IR

Problem: Rewriting the CFG mutably in-place leads to difficult-to-debug
situations, because it's hard to compare before and after. Instead, if an
e-graph IR is used, then all versions of nodes are retained and can be compared.

For now, keep the design of basic blocks owning memory and effects.

```rust
struct Graph {
    nodes: Vec<NodeEntry>,                  // NodeId -> Node
    node_ids: hashbrown::HashTable<NodeId>, // Node -> NodeId
    eclasses: Vec<Eclass>,                  // EclassId -> Eclass
}

struct NodeEntry {
    node: Node,
    hash: u64,
    eclass: EclassId,
    creator: Pass,
}

struct Eclass {
    canon: NodeId,
    nodes: Vec<NodeId>,
}

struct EclassId(NonZero<u32>);

enum Pass {
    Parse,
    AddLoopToMul,
    QuasiInvariantPeel,
    CopyConst,
    // …
}

enum Node {
    Block(Block),
    Seq(Vec<CtrlNode>),
    Loop(BoolNode, CtrlNode),
    If(BoolNode, CtrlNode, CtrlNode),

    Const(u8),
    Copy(Offset, BlockNode),
    Input(InputId),
    Add(ByteNode, ByteNode),
    Mul(ByteNode, ByteNode),

    IsZero(ByteNode),
    IsEven(ByteNode),
    True,
}

struct NodeId(NonZero<u32>);

// IDs for groups of node variants.
struct CtrlNode(NodeId);
struct ByteNode(NodeId);
struct BoolNode(NodeId);

// IDs for node variants.
struct BlockNode(NodeId);
struct SeqNode(NodeId);
struct LoopNode(NodeId);
struct IfNode(NodeId);
struct ConstNode(NodeId);
struct CopyNode(NodeId);
struct InputNode(NodeId);
struct AddNode(NodeId);
struct MulNode(NodeId);
struct IsZeroNode(NodeId);
struct IsEvenNode(NodeId);
struct TrueNode(NodeId);

struct InputId(u32);

struct Offset(i64);

struct Block {
    // …
    effects: Vec<Effect>,
}

enum Effect {
    Output(Vec<ByteNode>),
    Input(InputNode),
    GuardShift(Offset),
}
```

If effects are made nodes, then ordering needs to be enforced with `EffectNode`
tokens. This should also make them unique.

```rust
enum Node {
    // …
    Output(EffectNode, Vec<ByteNode>),
    Input(EffectNode),
    GuardShift(EffectNode, Offset),
}

struct EffectNode(NodeId); // Control or effect
```
