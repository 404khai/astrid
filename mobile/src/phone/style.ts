/** The chat icons and colours both apps share (DesktopCore `ChatStyle`). */
export const chatIcons: Record<string, string> = {
  chat: "message-outline",
  star: "star-outline",
  brain: "brain",
  code: "code-tags",
  book: "book-open-variant",
  heart: "heart-outline",
  bolt: "lightning-bolt-outline",
  globe: "earth",
  flask: "flask-outline",
  music: "music-note",
  house: "home-outline",
  camera: "camera-outline",
  map: "map-outline",
  leaf: "leaf",
};

export const chatColors: [string, string][] = [
  ["blue", "#4459F9"],
  ["violet", "#8A5CFF"],
  ["pink", "#E5487F"],
  ["red", "#E5484D"],
  ["orange", "#F07A2B"],
  ["yellow", "#E8B400"],
  ["green", "#3DBE6A"],
  ["teal", "#00C2A8"],
  ["grey", "#8E8E99"],
];

export const colorOf = (name?: string) => chatColors.find(([n]) => n === name)?.[1];
