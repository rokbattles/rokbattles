import { Button } from "@/components/ui/button";
import { Heading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";

export default function NotFound() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center px-6 py-24 text-center">
      <Text>404</Text>
      <Heading level={1} className="mt-4">
        Page not found
      </Heading>
      <Text className="mt-6">We couldn’t find the page you’re looking for.</Text>
      <Button href="/" variant="primary" className="mt-8">
        Go back home
      </Button>
    </div>
  );
}
