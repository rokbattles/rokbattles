import { Inter } from "next/font/google";
import "./globals.css";
import Image from "next/image";
import { ButtonLink, PlainButtonLink } from "@/components/elements/Button";
import { Main } from "@/components/elements/Main";
import { Navbar } from "@/components/sections/navbar/Navbar";
import { NavbarLink } from "@/components/sections/navbar/NavbarLink";
import { NavbarLogo } from "@/components/sections/navbar/NavbarLogo";

const inter = Inter({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-inter",
});

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={inter.variable}>
      <body>
        <>
          <Navbar
            id="navbar"
            logo={
              <NavbarLogo href="/">
                <Image
                  src="/img/logos/rokbattles-dark.svg"
                  alt="ROK Battles"
                  className="dark:hidden"
                  width={113}
                  height={28}
                />
                <Image
                  src="/img/logos/rokbattles-white.svg"
                  alt="ROK Battles"
                  className="not-dark:hidden"
                  width={113}
                  height={28}
                />
              </NavbarLogo>
            }
            links={
              <>
                <NavbarLink href="#">Pricing</NavbarLink>
                <NavbarLink href="#">Docs</NavbarLink>
                <NavbarLink href="#" className="sm:hidden">
                  Log in
                </NavbarLink>
              </>
            }
            actions={
              <>
                <PlainButtonLink href="#" className="max-sm:hidden">
                  Log in
                </PlainButtonLink>
                <ButtonLink href="#">Explore</ButtonLink>
              </>
            }
          />
          <Main>{children}</Main>
        </>
      </body>
    </html>
  );
}
