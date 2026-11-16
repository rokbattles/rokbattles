import type {
  Program,
  ProgramApplication,
  ProgramApplicationsResponse,
  ProgramDefinition,
} from "@/lib/(app)/types/program-application";

export const PROGRAM_APPLICATION_ACTION = "program_application";

export const PROGRAMS: readonly ProgramDefinition[] = [
  {
    id: "content_creator",
    label: "Content Creator",
    title: "Apply to the Content Creator Program",
    description: "Share your YouTube channel with us.",
    question: "YouTube Channel URL",
    input: "url",
    maxLength: 2048,
    placeholder: "https://www.youtube.com/@yourchannel",
    validationError:
      "Enter a valid HTTPS YouTube channel URL, such as https://www.youtube.com/@yourchannel.",
  },
  {
    id: "developer_program",
    label: "Developer Program",
    title: "Apply to the Developer Program",
    description: "Tell us about your idea and how ROK Battles would fit into it.",
    question: "What would you like to build with ROK Battles?",
    input: "textarea",
    maxLength: 5000,
    helpText:
      "Describe what you plan to build or how you would integrate ROK Battles. Up to 5,000 characters.",
    validationError: "Describe your plans in 5,000 characters or fewer.",
  },
];

export class ProgramApplicationRequestError extends Error {
  constructor(public status: number) {
    super("Program application request failed");
  }
}

const endpoint = "/proxy/v1/program-applications";

export async function fetchProgramApplications(
  signal?: AbortSignal
): Promise<ProgramApplication[]> {
  const response = await fetch(endpoint, { signal, cache: "no-store" });

  if (!response.ok) {
    throw new ProgramApplicationRequestError(response.status);
  }

  const payload = (await response.json()) as ProgramApplicationsResponse;

  return payload.applications;
}

export async function submitProgramApplication(
  program: Program,
  answer: string,
  turnstileToken: string
): Promise<ProgramApplication> {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ program, answer, turnstileToken }),
    cache: "no-store",
  });

  if (!response.ok) {
    throw new ProgramApplicationRequestError(response.status);
  }

  const payload = (await response.json()) as ProgramApplicationsResponse;
  const application = payload.applications[0];

  if (!application) {
    throw new Error("Application submission response is missing the application");
  }

  return application;
}
