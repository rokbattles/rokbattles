import { Link } from "../ui/link";

export function AppLogo() {
  return (
    <Link
      href="/app"
      aria-label="ROK Battles"
      className="w-fit cursor-pointer rounded-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-blue-500"
    >
      <img
        src="/assets/marketing/logo.svg"
        alt="ROK Battles"
        width={2104}
        height={556.24}
        className="h-auto w-36"
      />
    </Link>
  );
}
