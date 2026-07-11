import { useExtracted } from "next-intl";
import { Strong, Text } from "@/components/ui/text";

export function DrastcCredits() {
  const t = useExtracted();

  return (
    <div className="space-y-2">
      <div className="font-semibold text-sm text-zinc-950 dark:text-white">{t("Credits")}</div>
      <Text>
        {t.rich(
          "A scoring model created by ROK Battles, The King's Codex, and AQ/HQ. Designed by Davor and implemented by ROK Battles, the model is available through the <bold>ROK Battles: Combat Lab</bold> and the Discord communities for <bold>The King's Codex</bold> and <bold>AQ/HQ</bold>.",
          {
            bold: (chunks) => <Strong>{chunks}</Strong>,
          }
        )}
      </Text>
      <div className="flex flex-wrap gap-x-4 gap-y-1">
        <a
          className="inline-flex font-medium text-blue-600 text-sm/6 hover:text-blue-500 dark:text-blue-400 dark:hover:text-blue-300"
          href="https://discord.gg/kingscodex"
          target="_blank"
          rel="noopener noreferrer"
        >
          {t("The King's Codex")}
        </a>
        <a
          aria-label={t("Learn more about DRASTC")}
          className="inline-flex font-medium text-blue-600 text-sm/6 hover:text-blue-500 dark:text-blue-400 dark:hover:text-blue-300"
          href="https://buymeacoffee.com/davorrok/introducing-drastc"
          target="_blank"
          rel="noopener noreferrer"
        >
          {t("Learn more")}
        </a>
      </div>
    </div>
  );
}
