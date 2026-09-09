import { ArrowRight } from "lucide-react";
import Metadata from "../components/metadata";
import { AuthLayout } from "../components/ui/auth-layout";
import { Button } from "../components/ui/button";
import { Heading } from "../components/ui/heading";
import { Text } from "../components/ui/text";

export default function NotFoundRoute() {
  return (
    <AuthLayout>
      <Metadata title="404 Page Not Found" />
      <div className="max-w-xl text-center">
        <Text className="font-semibold text-orange-400!">404</Text>
        <Heading className="mt-4">Page not found</Heading>
        <Text className="mt-6">Sorry, we couldn’t find the page you’re looking for.</Text>
        <div className="mt-10 flex flex-wrap items-center justify-center gap-x-6 gap-y-4">
          <Button href="/" color="orange">
            Go back home
          </Button>
          <Button href="https://discord.gg/G33SzQgx6d" plain>
            Contact support
            <ArrowRight aria-hidden="true" />
          </Button>
        </div>
      </div>
    </AuthLayout>
  );
}
