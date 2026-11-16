import type { Metadata } from "next";

import {
  PageHeader,
  PageHeaderDescription,
  PageHeaderHeading,
} from "@/components/page-header";
import promise1 from "@/assets/promise-1.webp";
import promise2 from "@/assets/promise-2.webp";
import promise3 from "@/assets/promise-3.webp";
import promise4 from "@/assets/promise-4.webp";
import promise5 from "@/assets/promise-5.webp";
import promise6 from "@/assets/promise-6.webp";
import promise7 from "@/assets/promise-7.webp";
import promise8 from "@/assets/promise-8.webp";
import Image, { StaticImageData } from "next/image";
import {
  ArrowBigUpDash,
  SquareArrowOutUpLeft,
  SquareArrowOutUpRight,
  SquareArrowRight,
} from "lucide-react";

export const metadata: Metadata = {
  title: "Showcase",
  description:
    "The applications of the repository: each exists to prove one thing, and says which.",
};

type Vitrine = {
  name: string;
  description: string;
  inpired_by: { name: string; url: string };
  what: string;
  related: string[];
  image: StaticImageData;
};

const APPLICATIONS: Vitrine[] = [
  {
    name: "AI Agent Manager",
    description: "A software for deploy manage your AI agents.",
    inpired_by: { name: "Design Grid", url: "https://tiktok.com/@designgrid" },
    what: "Recharts JS, TailwindCSS, Geist Font, Lucide React, Supabase, LLM-Studio/Anthropic/OpenRouter API. Reqwest, Axum,Serde... All those are the dependencies has been used for build this app",
    related: ["Recharts", "Shadcn", "async fn main", "+ features"],
    image: promise1,
  },
  {
    name: "R Intelligent Data",
    description: "A full GUI for Databases Management",
    inpired_by: { name: "Design Grid", url: "https://tiktok.com/@designgrid" },
    what: " The power of shadcn tables & other components joined to Rust Git2 crate + Docker CLI + Kubernetes client, a controller runtime, a custom resource derive, and various tooling required for building applications or controllers that interact with Kubernetes.",
    related: ["Drag&Drop", "DevOps", "Shadcn UI", "Database", "Scale Up"],
    image: promise2,
  },
  {
    name: "Vault-Storage Secure",
    description: "Hyper Secret data storage",
    inpired_by: { name: "Design Grid", url: "https://tiktok.com/@designgrid" },
    what: "Cyber deck Font, TailwindCSS, QrCode crate. They were joined to give birth to this wonderfull projet. Btw his code is available on our public repo !",
    related: ["Encryption", "HyperFrames", "Algorithms"],
    image: promise3,
  },
  {
    name: "Monitor",
    description: "Remote Ressource App for Agents.",
    inpired_by: { name: "Design Grid", url: "https://tiktok.com/@designgrid" },
    what: "This is a remote hardware tracking desktop app, implemented by Claude Code to follow all the agents usages: CPU,GPU,RAM,ROM,NETWORK,TOKENS+ and alert a human on Email/Whatsapp .",
    related: ["Tasks", "Ressources", "Optimisation"],
    image: promise4,
  },
  {
    name: "MultiTask Hub",
    description: "Tracks & Solve orchestration problems.",
    inpired_by: { name: "Design Grid", url: "https://tiktok.com/@designgrid" },
    what: "This application exists to solve any problem that you can encouter while scaling many projects at the same time. Was built with SeaOrm-CLI -> Supabase, Git2 crate, std, Shadcn, Tailwind v4, Reqwest crate... ",
    related: ["Systemm", "Routes", "Advanced", "Orchestration"],
    image: promise5,
  },
  {
    name: "Utility Hub",
    description: "Management App for Tech & Fin Startups. ",
    inpired_by: { name: "Utility Hub", url: "https://utility-app.xyz" },
    what: "An Application tought for growing up business that allow the team to follow each path of the Startup: Tech ( Git,Slack like Chat, Webhooks,Email/Whatsapp/In-App notifications, Smart & AI powered Automations, Driver,...) | Financial: [ Html to Email or Whatsapp Invoice/quote/receipt, ...] -> [The Client], Auto & Omniscient AI ...Dependencies: Convex crate, OpenRouter, Recharts, Git2 crate, Lucide React, Shadcn...",
    related: ["Finance", "Tech", "AI Powered", "OpenRouter"],
    image: promise6,
  },
  {
    name: "Nemo Design",
    description: "3D Agents powered App.",
    inpired_by: { name: "Dribbble", url: "https://dribbble.com/" },
    what: "A desktop application build for edit 3D models , Blender like app, ameliorated by AI agents, like Opus/Kimi/Grok/Astra to perform designing operations. Used: ThreeJS, Shadcn, Ffmpeg, Rodio, OpenRouter, Chrono Crate....",
    related: ["ThreeJS", "Reqwest", "Ffmpeg", "OpenRouter"],
    image: promise7,
  },
  {
    name: "Altlassian Raw",
    description: "Tiny video editor app ffmpeg powered by.",
    inpired_by: { name: "Dribbble", url: "https://dribbble.com/" },
    what: "A tiny video editor for small, fast cuts, resize, colorize, AI edits, subtitling. This app was made with Ffmpeg, joined to some React UI components",
    related: ["ThreeJS", "Shadcn", "Ffmpeg", "OpenRouter"],
    image: promise8,
  },
];

export default function ShowcasePage() {
  return (
    <div className="flex flex-1 flex-col">
      <PageHeader>
        <PageHeaderHeading>The showcase</PageHeaderHeading>
        <PageHeaderDescription>
          The applications of the repository serve as proof for the plans. Each
          exists for a specific reason, and says which. No screenshots here:
          what matters is what each one demonstrates.
        </PageHeaderDescription>
      </PageHeader>

      <section className="border-t border-grid pb-16">
        <div className="">
          <div className=" grid gap-px bg-border/60 lg:grid-cols-2">
            {APPLICATIONS.map((application) => (
              <article
                key={application.name}
                className="flex flex-col gap-4 bg-background p-6 lg:p-8"
              >
                <header className="flex flex-col   ">
                  <div className="flex flex-row gap-4 items-center">
                    <h2 className="font-mono text-lg font-semibold tracking-tight">
                      {application.name}
                    </h2>
                    <p className="text-sm text-muted-foreground">
                      {" "}
                      {application.description}{" "}
                    </p>
                  </div>
                  <p className="text-sm text-muted-foreground flex flex-row items-center gap-1">
                    {" "}
                    Inpired by
                    <a
                      href={application.inpired_by.url}
                      className="hover:text-white flex flex-row items-center gap-1"
                    >
                      {application.inpired_by.name}
                      <SquareArrowOutUpRight size={12} />
                    </a>
                  </p>
                </header>
                <Image alt={application.name} src={application.image} />
                <p className="border-l-2 border-border pl-4 text-sm leading-relaxed text-muted-foreground">
                  {application.what}
                </p>
                <ul className="mt-auto flex flex-wrap gap-1.5 pt-2">
                  {application.related.map((mot) => (
                    <li
                      key={mot}
                      className="rounded-full border border-border bg-surface px-2.5 py-0.5 font-mono text-[0.7rem] text-muted-foreground"
                    >
                      {mot}
                    </li>
                  ))}
                </ul>
              </article>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
