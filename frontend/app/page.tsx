import { Header } from "@/components/layout/header";
import { PixelCanvas } from "@/components/canvas/pixel-canvas";

export default function Home() {
  return (
    <div className="flex min-h-screen flex-col">
      <Header />
      <main className="flex flex-1 items-center justify-center p-4">
        <PixelCanvas />
      </main>
    </div>
  );
}
