import { unstable_extractMessages } from "next-intl/extractor";

await unstable_extractMessages({
  srcPath: "./src",
  messages: {
    path: "./src/i18n/messages",
    format: "po",
    locales: "infer",
    sourceLocale: "en",
  },
});

console.log("Success");
