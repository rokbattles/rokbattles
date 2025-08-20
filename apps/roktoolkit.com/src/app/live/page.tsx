export default function LivePage() {
  return (
    <div className="relative isolate flex min-h-svh w-full max-lg:flex-col bg-zinc-900 lg:bg-zinc-950">
      <div className="fixed inset-y-0 left-0 w-96 max-lg:hidden">Live feed</div>
      <main className="flex flex-1 flex-col lg:min-w-0 lg:pl-96">
        <div className="grow lg:bg-zinc-900 lg:shadow-xs lg:ring-1 lg:ring-white/10">
          Battle report
        </div>
      </main>
    </div>
  );
}
