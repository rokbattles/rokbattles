import embed from "@/content/discord-embed.json";

export function DiscordEmbed() {
  return (
    <script
      id="discord:component-embed"
      type="application/json"
      // biome-ignore lint/security/noDangerouslySetInnerHtml: static JSON, escaped to prevent closing the script tag.
      dangerouslySetInnerHTML={{ __html: JSON.stringify(embed).replaceAll("<", "\\u003c") }}
    />
  );
}
