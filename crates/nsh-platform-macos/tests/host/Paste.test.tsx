// A paste into an input with `pasteBehavior="reportFiles"` on the macOS
// host (`nsh test crates/nsh-platform-macos/tests/host --target macos`, from
// the repository's root: the pasted image is read from there): AppKit's
// paste inserts the text of a paste that holds only text, and nothing of
// one that holds an image or a file, which the app hears of.
import { expect, open, test } from "nsh/test";

test("a paste with an image or a file inserts no text, a paste of text does", async () => {
  const app = await open("./Paste.tsx");
  await app.paste("label:Message", { text: "hello" });
  await app.waitFor("Text: hello");
  await app.waitFor("Pasted: hello files 0 uris 0");
  await app.paste("label:Message", { text: "shot.png", files: ["examples/gallery/icon.png"] });
  await app.waitFor("Pasted: shot.png files 1 uris 0");
  await app.paste("label:Message", { text: "notes", uris: ["file:///tmp/notes.txt"] });
  await app.waitFor("Pasted: notes files 0 uris 1");
  // An inserted text's change would have come by now.
  await app.wait(200);
  expect(app.shows("Text: hello")).toBe(true);
});
