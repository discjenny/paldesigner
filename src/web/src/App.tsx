import { DatabaseZap, Pickaxe } from 'lucide-react'
import { Button } from '@/components/ui/button'

function App() {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-4xl flex-col gap-8 px-6 py-16">
      <header className="space-y-3">
        <h1 className="text-3xl font-semibold tracking-tight">Paldesigner</h1>
        <p className="text-muted-foreground">
          Rust API + PostgreSQL backend with a Bun/Vite React frontend.
        </p>
      </header>

      <section className="grid gap-4 md:grid-cols-2">
        <article className="rounded-lg border bg-card p-5">
          <div className="mb-3 flex items-center gap-2 text-sm text-muted-foreground">
            <DatabaseZap className="h-4 w-4" />
            Backend
          </div>
          <p className="text-sm">Health endpoints: <code>/health</code> and <code>/ready</code>.</p>
        </article>
        <article className="rounded-lg border bg-card p-5">
          <div className="mb-3 flex items-center gap-2 text-sm text-muted-foreground">
            <Pickaxe className="h-4 w-4" />
            Frontend
          </div>
          <p className="text-sm">Tailwind v4 + shadcn foundation + Lucide icons enabled.</p>
        </article>
      </section>

      <div className="flex gap-3">
        <Button>Import Save ZIP</Button>
        <Button variant="outline">Open Planner</Button>
      </div>
    </main>
  )
}

export default App
