import { Strong, Text } from "@/components/ui/text";

export function DrastcCredits() {
  return (
    <div className="space-y-2">
      <div className="font-semibold text-sm text-white">Credits</div>
      <Text>
        A scoring model created by ROK Battles, The King's Codex, and AQ/HQ. Designed by Davor and
        implemented by ROK Battles, the model is available through the{" "}
        <Strong>ROK Battles: Combat Lab</Strong> and the Discord communities for{" "}
        <Strong>The King's Codex</Strong> and <Strong>AQ/HQ</Strong>.
      </Text>
      <div className="flex flex-wrap gap-x-4 gap-y-1">
        <a
          className="inline-flex font-medium text-sm/6 text-blue-400 hover:text-blue-300"
          href="https://discord.gg/kingscodex"
          target="_blank"
          rel="noopener noreferrer"
        >
          The King's Codex
        </a>
        <a
          className="inline-flex font-medium text-sm/6 text-blue-400 hover:text-blue-300"
          href="https://discord.gg/KP3dsGgsBA"
          target="_blank"
          rel="noopener noreferrer"
        >
          AQ/HQ
        </a>
        <a
          aria-label="Learn more about DRASTC"
          className="inline-flex font-medium text-sm/6 text-blue-400 hover:text-blue-300"
          href="https://buymeacoffee.com/davorrok/introducing-drastc"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn more
        </a>
      </div>
    </div>
  );
}
