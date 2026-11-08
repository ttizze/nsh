// A selectable view that shows where its selection is, as a bar of actions
// above the selection needs it. `Select.test.tsx` runs it on the macOS host.
import { mount, signal, Text, View } from "nsh";

function App() {
  const selected = signal("none");
  const text = signal("");
  const menu = signal("none");
  return (
    <View style={{ flex: 1, padding: 16, gap: 8, backgroundColor: "white" }}>
      <View
        selectable
        style={{ marginTop: 40, marginLeft: 30, width: 300, gap: 12, alignItems: "flex-start" }}
        onContextMenu={() => menu.set(`for ${JSON.stringify(text())}`)}
        onSelectionChange={({ text: t, bounds, pageBounds }) => {
          text.set(t);
          const sel = t;
          if (!bounds || !pageBounds) {
            selected.set(sel === "" ? "none" : `${sel} without bounds`);
            return;
          }
          const inside =
            bounds.origin.x >= 0 && bounds.origin.y >= 0 && bounds.size.width > 0 && bounds.size.height > 0;
          // The page box is the view's box moved by where the view is.
          const moved = pageBounds.origin.x - bounds.origin.x === 46 && pageBounds.origin.y - bounds.origin.y === 56;
          const tall = bounds.size.height > 20 ? "two lines" : "one line";
          selected.set(`${sel.length} chars, ${inside ? "inside" : "outside"}, ${moved ? "moved" : "not moved"}, ${tall}`);
        }}
      >
        <Text>Alpha beta gamma</Text>
        <Text>Delta epsilon</Text>
      </View>
      <Text>{`Selected: ${selected()}`}</Text>
      <Text>{`Menu: ${menu()}`}</Text>
    </View>
  );
}

mount(App);
