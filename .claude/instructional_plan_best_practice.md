# Best practices for instructional implementation plans

A plan here is not just a task list — it doubles as learning material. The reader should finish a
slice with working software **and** a deeper understanding of every pattern they typed.

## Ask the user
Begin with an iterative process to figure out what technical concepts, techniques, languages, 
workflows, etc. the user is and is not familiar with. Ask as many questions as it takes to build
up a model of their knowledge gaps, to make the guide as instructional as possible in the way the
user wants to learn. This is the most important step, and it is better to ask more questions than it
is to ask fewer question.

## Structure

- **Slice, don't layer.** Each plan file is one vertical slice ending in something you can *see
  working*. Open with: **Goal** (one paragraph), **Outcome** (the observable result),
  **New ideas** (a scannable list of concepts this slice introduces), **Starting point** (what must
  already work).
- **Steps are small, numbered, and checkable** (`- [ ]`). One step = one file touched or one command
  run. Every code step is followed by a run/verify step with the **expected output stated**
  ("Expected: 3 passed", "Expect `count = 122497`").
- **TDD where it fits:** write the failing test → run it, watch it fail (state the expected failure) →
  write the code → run it, watch it pass. The fail step is not optional ceremony; it proves the test
  tests something.
- **End each step with a manual verification step when possible** ("Look at it") and a commit step with the exact command and message. It's best to get the user to run the code to see what they have built.
- **Iterate**. When presenting concepts or presenting code a user is unfamiliar with, it's better to start simple and iteratively build up complexity, rather than presenting the complete final code up front. This will help the user understand *why* certain things are needed.

## Presenting code

- **Start with a skeleton.** The user wants to learn, so let them build the code on their own, such as "build a function that takes X and returns Y"
- **Suggest how you would do it** Show your suggested code, then a numbered **"explanation"** section: each piece is the next typeable chunk (5–20 lines) plus 1–3 sentences of *why*.
  The reader constructs the file by following pieces in order; the full block is the target, not the
  instructions. Never dump >~40 lines or multiple functions without this.
- **Fold explanation into the piece it explains.** No separate "Walkthrough" bullets duplicating what
  a piece's caption says.
- **Code must be complete and copy-pasteable** — no `# ...` elisions inside a piece. When editing an
  existing file, show the anchor ("add after `labels_for`", "replace the `patches` key with:").
- **Build iteratively.** Don't just present the instructions for a code block top to bottom, build it
  iteratively, and introduce things (like imports, constants, etc) when they are needed, with
  explanation.
- **Explain the code.** If, in the questions, it is obvious that the user doesn't really know the language
  or framework or library, you should explain the code nearly line by line.

## Explaining

- **Assume the reader's actual baseline, not a convenient one.** When unsure whether they know a
  pattern, *ask* (a multi-select "which of these do you already know?" works well) rather than
  guessing in either direction.
- **Point of use, never a primer block.** Explain a concept as an aside inside the piece where it
  is first typed — not in a big foundations section up front. A wall of eight concepts before any code
  is as non-iterative as a wall of code.
- **First use explains; later uses lightly reference with cross-references.** Track where each concept was introduced and
  point back ("the list-with-`key` pattern from Slice 4, Step 16") instead of re-explaining.
- **Prefer idiomatic code over consistency with the reader's current habits — but never silently.**
  If the idiomatic form uses something non-obvious (property shorthand, `as const`, an IIFE), the
  aside says why the odd-looking thing is the normal thing.
- **Explain the *why this design*, not just the *what***

## Updating
- The use may make changes to your design choices, you should be able to update the remaining plan to reflect these choices

## Tone and scope
- Prefer ASD-STE100 english, but you can add technical jargon. ISO 24495-1 is also fine.
- **Analogies and references to python can help.** But don't overuse analogies, or make really
  poor ones.
- **Being concise is good.** But the terse, parallel structure with lots of lists and em dashes is
  annoying to read.
