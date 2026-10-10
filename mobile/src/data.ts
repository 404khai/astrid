export type Thread = {
  id: string;
  title: string;
  model: string;
  branch: string;
  time: string;
  working?: boolean;
};
export type Project = {
  id: string;
  name: string;
  avatar: number;
  threads: Thread[];
};
export const projects: Project[] = [
  {
    id: "astrid",
    name: "astrid",
    avatar: require("../assets/avatars/download (1).png"),
    threads: [
      {
        id: "a1",
        title: "Audit Terminal UI Migration",
        model: "GPT-6.1-Sol",
        branch: "astrid/feat/cli",
        time: "Now",
        working: true,
      },
      {
        id: "a2",
        title: "Add Observability Settings",
        model: "Opus 5.5",
        branch: "astrid/feat/observability",
        time: "3h 47m",
      },
    ],
  },
  {
    id: "keihatsu",
    name: "Keihatsu",
    avatar: require("../assets/avatars/download (2).png"),
    threads: [
      {
        id: "k1",
        title: "Add iOS Blobatar Avatars",
        model: "Mistral Large 4",
        branch: "Keihatsu/feat/ios-blobatar-integration",
        time: "3h 47m",
      },
      {
        id: "k2",
        title: "Implement CBZ Manga Downloads",
        model: "MiMo-V2.6-Flash Free",
        branch: "Keihatsu/feat/downloads-and-storage",
        time: "3h 47m",
      },
    ],
  },
  {
    id: "roadrunner",
    name: "roadrunner",
    avatar: require("../assets/avatars/download (3).png"),
    threads: [
      {
        id: "r1",
        title: "Refine the reading experience",
        model: "GPT-6.1-Sol",
        branch: "roadrunner/feat/reader",
        time: "Yesterday",
      },
      {
        id: "r2",
        title: "Check offline sync",
        model: "Opus 5.5",
        branch: "roadrunner/fix/sync",
        time: "Yesterday",
      },
    ],
  },
  {
    id: "wisp",
    name: "wisp",
    avatar: require("../assets/avatars/download (4).png"),
    threads: [
      {
        id: "w1",
        title: "Polish the command menu",
        model: "GPT-6.1-Sol",
        branch: "wisp/feat/commands",
        time: "Yesterday",
      },
      {
        id: "w2",
        title: "Review keyboard shortcuts",
        model: "Opus 5.5",
        branch: "wisp/feat/shortcuts",
        time: "Yesterday",
      },
      {
        id: "w3",
        title: "Explore a quieter interface",
        model: "GPT-6.1-Sol",
        branch: "wisp/design",
        time: "Monday",
      },
    ],
  },
];
