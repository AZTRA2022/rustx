The React components this application owns. `npx shadcn add button`
writes into `components/ui/`; anything you write yourself goes here
beside it.

A page writes the file's name as a tag — `components/Editor.tsx` is
`<Editor />` — and the framework builds the index that makes that
true at every build. Nothing here has to be declared anywhere.

`tsconfig.json` compiles this directory and `components.json` calls
it `@/components`, so an import of `@/components/ui/button` resolves
for the shadcn command line, for esbuild and for an editor alike.
