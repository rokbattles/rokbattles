import { Heading } from "@/components/ui/heading";
import { TextLink } from "@/components/ui/text";

export default function Page() {
  return (
    <>
      <Heading>ROK Battles Terms & Policies</Heading>
      <ul className="mt-2 ml-4 list-disc">
        <li>
          <TextLink href="/legal/terms-of-service">Terms of Service</TextLink>
        </li>
        <li>
          <TextLink href="/legal/privacy-policy">Privacy Policy</TextLink>
        </li>
        <li>
          <TextLink href="/legal/cookie-policy">Cookie Policy</TextLink>
        </li>
      </ul>
    </>
  );
}
