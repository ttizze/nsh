// A message field that inserts pasted text but leaves pasted images and
// files to the app (`pasteBehavior="reportFiles"`). `Paste.test.tsx` runs
// it on the macOS host.
import { mount, signal, Text, View } from "nsh";
import { Input } from "nsh/base";

function App() {
  const text = signal("");
  const pasted = signal("none");
  return (
    <View style={{ flex: 1, padding: 16, gap: 8 }}>
      <Input
        accessibilityLabel="Message"
        pasteBehavior="reportFiles"
        style={{ width: 300, height: 40 }}
        onChange={({ value }) => text.set(value)}
        onPaste={(e) => pasted.set(`${e.text} files ${e.files.length} uris ${e.uris.length}`)}
      />
      <Text>{`Text: ${text()}`}</Text>
      <Text>{`Pasted: ${pasted()}`}</Text>
    </View>
  );
}

mount(App);
