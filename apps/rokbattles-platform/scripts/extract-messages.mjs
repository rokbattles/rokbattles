import { unstable_extractMessages } from "next-intl/extractor";

await unstable_extractMessages({
  srcPath: "./src",
  sourceLocale: "en",
  messages: {
    path: "./src/i18n/messages",
    format: "po",
    locales: "infer",
  },
});

console.log("Success");
