export type Program = "content_creator" | "developer_program";

export type ProgramApplication = {
  id: string;
  program: Program;
  status: "open";
  createdAt: number;
};

export type ProgramApplicationsResponse = {
  applications: ProgramApplication[];
};

export type ProgramDefinition = {
  id: Program;
  label: string;
  title: string;
  description: string;
  question: string;
  input: "url" | "textarea";
  maxLength: number;
  placeholder?: string;
  helpText?: string;
  validationError: string;
};
