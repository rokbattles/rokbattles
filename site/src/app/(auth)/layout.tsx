import { ThemeProvider } from "next-themes";
import { AuthLayout } from "@/components/ui/auth-layout";

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
      <AuthLayout>{children}</AuthLayout>
    </ThemeProvider>
  );
}
