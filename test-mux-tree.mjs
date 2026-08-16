// Ad-hoc unit test for multiplexer tree logic (mirrors Terminal.svelte)
// Run: node test-mux-tree.mjs

function leaf(paneId) { return { type: "leaf", paneId }; }

function findParentNode(node, target) {
  if (!node || node.type === "leaf") return null;
  for (const c of node.children) {
    if (c === target) return node;
    if (c.type === "leaf" && c.paneId === target) return node;
    const r = findParentNode(c, target);
    if (r) return r;
  }
  return null;
}

function collectPaneIds(node, out = []) {
  if (!node) return out;
  if (node.type === "leaf") out.push(node.paneId);
  else node.children.forEach((c) => collectPaneIds(c, out));
  return out;
}

let paneSeq = 1;
function nextPaneId() { return `p${paneSeq++}`; }

function splitPane(root, paneId, dir) {
  const parent = findParentNode(root, paneId);
  if (parent && parent.type === "branch" && parent.dir === dir) {
    const newPane = nextPaneId();
    parent.children.push(leaf(newPane));
    parent.ratio = 1 / parent.children.length;
    return newPane;
  }
  const newPane = nextPaneId();
  const branch = { type: "branch", dir, ratio: 0.5, children: [leaf(paneId), leaf(newPane)] };
  if (!parent) {
    Object.assign(root, branch);
  } else {
    const idx = parent.children.findIndex((c) => c.type === "leaf" && c.paneId === paneId);
    if (idx !== -1) parent.children[idx] = branch;
  }
  return newPane;
}

function closePane(root, paneId) {
  const parent = findParentNode(root, paneId);
  if (!parent) return false;
  const idx = parent.children.findIndex((c) => c.type === "leaf" && c.paneId === paneId);
  if (idx === -1) return false;
  parent.children.splice(idx, 1);
  if (parent.children.length === 1) {
    const only = parent.children[0];
    const grandparent = findParentNode(root, parent);
    if (!grandparent) {
      Object.keys(root).forEach((k) => delete root[k]);
      Object.assign(root, only);
    } else {
      const gi = grandparent.children.findIndex((c) => c === parent);
      grandparent.children[gi] = only;
    }
  } else {
    parent.ratio = 1 / parent.children.length;
  }
  return true;
}

// ---- tests ----
let pass = 0, fail = 0;
function assert(cond, msg) {
  if (cond) { pass++; console.log(`  ok: ${msg}`); }
  else { fail++; console.log(`  FAIL: ${msg}`); }
}

console.log("T1: split root leaf");
{
  paneSeq = 1;
  const root = leaf(nextPaneId()); // p1
  const np = splitPane(root, root.paneId, "row");
  assert(np === "p2", `new pane p2 (got ${np})`);
  assert(root.type === "branch" && root.dir === "row", "root becomes row branch");
  assert(root.children.length === 2, "2 children");
  assert(collectPaneIds(root).join(",") === "p1,p2", `ids p1,p2 (got ${collectPaneIds(root).join(",")})`);
}

console.log("T2: split p1 again (vertical) → nested");
{
  paneSeq = 1;
  const root = leaf(nextPaneId()); // p1
  const p1 = root.paneId;
  splitPane(root, p1, "row"); // row[p1,p2]
  const np = splitPane(root, p1, "col"); // p1 → col[p1,p3]
  assert(np === "p3", `new pane p3 (got ${np})`);
  assert(root.type === "branch" && root.dir === "row", "root still row");
  assert(root.children[0].type === "branch" && root.children[0].dir === "col", "left child is col branch");
  assert(collectPaneIds(root).join(",") === "p1,p3,p2", `ids p1,p3,p2 (got ${collectPaneIds(root).join(",")})`);
}

console.log("T3: split same direction appends to branch");
{
  paneSeq = 1;
  const root = leaf(nextPaneId()); // p1
  const p1 = root.paneId;
  splitPane(root, p1, "row"); // row[p1,p2]
  splitPane(root, p1, "row"); // appends → row[p1,p2,p4]
  assert(root.children.length === 3, `3 children in row (got ${root.children.length})`);
  assert(collectPaneIds(root).length === 3, "3 panes");
}

console.log("T4: close pane in middle");
{
  paneSeq = 1;
  const root = leaf(nextPaneId()); // p1
  const p1 = root.paneId;
  splitPane(root, p1, "row"); // row[p1,p2]
  splitPane(root, p1, "row"); // row[p1,p2,p4]
  const ok = closePane(root, p1);
  assert(ok === true, "close succeeds");
  assert(collectPaneIds(root).length === 2, `2 panes left (got ${collectPaneIds(root).length})`);
  assert(root.type === "branch" && root.children.length === 2, "branch has 2 children");
}

console.log("T5: close to single pane → collapse to leaf");
{
  paneSeq = 1;
  const root = leaf(nextPaneId()); // p1
  const p1 = root.paneId;
  splitPane(root, p1, "row"); // row[p1,p2]
  closePane(root, "p2");
  assert(root.type === "leaf" && root.paneId === p1, `root collapses to leaf ${p1} (got ${root.type}:${root.paneId})`);
}

console.log("T6: nested close collapses grandchild");
{
  paneSeq = 1;
  const root = leaf(nextPaneId()); // p1
  const p1 = root.paneId;
  splitPane(root, p1, "row"); // row[p1,p2]
  splitPane(root, p1, "col"); // row[col[p1,p3], p2]
  closePane(root, "p3"); // col[p1,p3] → p1 → row[p1,p2]
  assert(root.type === "branch" && root.dir === "row", "root still row");
  assert(root.children.length === 2, "2 children");
  assert(collectPaneIds(root).join(",") === "p1,p2", `p1,p2 remain (got ${collectPaneIds(root).join(",")})`);
}

console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
