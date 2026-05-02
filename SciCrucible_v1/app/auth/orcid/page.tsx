import { OrcidGatePage } from "@/components/orcid-gate"
import { GlobalNav } from "@/components/global-nav"

export default function OrcidAuthPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />
      <main className="flex-1 ml-64 min-h-screen flex flex-col">
        <OrcidGatePage />
      </main>
    </div>
  )
}
