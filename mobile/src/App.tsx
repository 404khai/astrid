import React, { useRef, useState } from "react";
import {
  Image,
  ImageBackground,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
  ScrollView as RNScrollView,
  KeyboardAvoidingView,
} from "react-native";
import { StatusBar } from "expo-status-bar";
import { SymbolView, type SymbolViewProps } from "expo-symbols";
import {
  Host,
  RNHostView,
  TabView,
  NavigationStack,
  ScrollView,
  Toolbar,
  ToolbarItem,
  Button,
  VStack,
  ZStack,
} from "@expo/ui/swift-ui";
import {
  navigationTitle,
  navigationBarTitleDisplayMode,
  scrollEdgeEffectStyle,
  useScrollGeometryChange,
  tint,
} from "@expo/ui/swift-ui/modifiers";
import { NativeSheet } from "./NativeSheet";
import { projects as initialProjects, type Thread } from "./data";
import { s } from "./styles";
const blue = "#6264ff";
const logo = require("../assets/logo.png");
function Icon({
  name,
  color = "#92929e",
  size = 20,
}: {
  name: SymbolViewProps["name"];
  color?: string;
  size?: number;
}) {
  return (
    <SymbolView
      name={name}
      tintColor={color}
      style={{ width: size, height: size }}
    />
  );
}
export default function App() {
  const [tab, setTab] = useState("Home");
  const [projects, setProjects] = useState(initialProjects);
  const [expanded, setExpanded] = useState(["astrid", "keihatsu"]);
  const [sheet, setSheet] = useState(false);
  const [detail, setDetail] = useState<Thread | null>(null);
  const [filter, setFilter] = useState(false);
  const [archived, setArchived] = useState(false);
  const [search, setSearch] = useState("");
  const [prompt, setPrompt] = useState("");
  const [project, setProject] = useState("astrid");
  const [model, setModel] = useState("GPT-6.1-Sol");
  const working = projects
    .flatMap((p) => p.threads)
    .filter((t) => t.working).length;
  function submit() {
    if (!prompt.trim()) return;
    const thread = {
      id: String(Date.now()),
      title: prompt.trim(),
      model,
      branch: `${project}/main`,
      time: "Just now",
    };
    setProjects((ps) =>
      ps.map((p) =>
        p.id === project ? { ...p, threads: [thread, ...p.threads] } : p,
      ),
    );
    setExpanded((e) => [...new Set([...e, project])]);
    setPrompt("");
    setSheet(false);
    setTab("Home");
    setDetail(thread);
  }
  function content(page: string) {
    if (page === "Settings")
      return (
        <View style={s.detail}>
          <Text style={s.detailTitle}>Made for your Astrid.</Text>
          <Text style={s.body}>Mobile concept · iOS first</Text>
          <View style={s.message}>
            <Icon name="desktopcomputer" />
            <Text style={s.body}>Runtime connection · Not connected</Text>
          </View>
          <Text style={s.eyebrow}>
            Demo sessions stay in memory. No model calls or repository changes.
          </Text>
        </View>
      );
    if (detail && page === "Home")
      return (
        <View style={s.detail}>
          <Pressable onPress={() => setDetail(null)}>
            <Text style={{ color: blue }}>‹ All sessions</Text>
          </Pressable>
          <Text style={s.eyebrow}>
            {detail.model} · {detail.branch}
          </Text>
          <Text style={s.detailTitle}>{detail.title}</Text>
          <View style={s.message}>
            <Image source={logo} style={{ width: 25, height: 20 }} />
            <Text style={s.body}>
              Your prompt stays on this device. Connect an Astrid runtime to
              execute this session.
            </Text>
          </View>
        </View>
      );
    return (
      <View>
        {page === "Search" && (
          <TextInput
            style={s.search}
            value={search}
            onChangeText={setSearch}
            placeholder="Search sessions or projects"
            placeholderTextColor="#858594"
            accessibilityLabel="Search sessions"
          />
        )}
        {archived ? (
          <Text style={s.empty}>No archived sessions yet</Text>
        ) : (
          projects.map((p) => {
            const threads = p.threads.filter(
              (t) =>
                (!filter || t.working) &&
                `${t.title} ${p.name}`
                  .toLowerCase()
                  .includes(page === "Search" ? search.toLowerCase() : ""),
            );
            if (!threads.length) return null;
            const open = expanded.includes(p.id) || page === "Search" || filter;
            return (
              <View key={p.id}>
                <Pressable
                  accessibilityRole="button"
                  accessibilityState={{ expanded: open }}
                  onPress={() =>
                    setExpanded((e) =>
                      e.includes(p.id)
                        ? e.filter((id) => id !== p.id)
                        : [...e, p.id],
                    )
                  }
                  style={s.project}
                >
                  <Image source={p.avatar} style={s.avatar} />
                  <Text style={s.projectName}>{p.name}</Text>
                  <Text style={s.count}>{threads.length}</Text>
                  <Icon
                    name={open ? "chevron.down" : "chevron.right"}
                    size={12}
                  />
                </Pressable>
                {open &&
                  threads.map((t) => (
                    <Pressable
                      key={t.id}
                      style={s.thread}
                      onPress={() => {
                        setDetail(t);
                        setTab("Home");
                      }}
                      accessibilityRole="button"
                    >
                      <View style={s.row}>
                        <Image source={logo} style={s.modelLogo} />
                        <Text style={s.model}>{t.model}</Text>
                        <View style={{ flex: 1 }} />
                        {t.working ? (
                          <Text style={s.badge}>Working</Text>
                        ) : (
                          <Text style={s.time}>{t.time}</Text>
                        )}
                      </View>
                      <Text numberOfLines={1} style={s.threadTitle}>
                        {t.title}
                      </Text>
                      <View style={s.row}>
                        <Icon name="arrow.triangle.branch" size={12} />
                        <Text numberOfLines={1} style={s.branch}>
                          {t.branch}
                        </Text>
                      </View>
                    </Pressable>
                  ))}
              </View>
            );
          })
        )}
        {page === "Search" &&
          !projects.some((p) =>
            p.threads.some((t) =>
              `${t.title} ${p.name}`
                .toLowerCase()
                .includes(search.toLowerCase()),
            ),
          ) && <Text style={s.empty}>No matching sessions</Text>}
      </View>
    );
  }
  return (
    <View style={s.root}>
      <StatusBar style="light" />
      <ImageBackground
        source={require("../assets/background.png")}
        style={StyleSheet.absoluteFill}
        resizeMode="cover"
      >
        <View style={s.shade} />
      </ImageBackground>
      <Host style={{ flex: 1 }} colorScheme="dark">
        <TabView
          selection={tab}
          onSelectionChange={setTab}
          modifiers={[tint(blue)]}
        >
          {(["Home", "Settings", "Search"] as const).map((page) => (
            <TabView.Tab
              key={page}
              value={page}
              label={page}
              systemImage={
                page === "Home"
                  ? "house"
                  : page === "Settings"
                    ? "gearshape"
                    : "magnifyingglass"
              }
            >
              <NativePage
                page={page}
                title={detail && page === "Home" ? "Session" : page}
                onFilter={() => setFilter(!filter)}
                onArchive={() => setArchived(!archived)}
              >
                {content(page)}
              </NativePage>
            </TabView.Tab>
          ))}
        </TabView>
      </Host>
      {tab !== "Settings" && (
        <View
          pointerEvents="box-none"
          style={{ position: "absolute", left: 20, right: 20, bottom: 96 }}
        >
          <Pressable
            accessibilityRole="button"
            onPress={() => setSheet(true)}
            style={s.newSession}
          >
            <Icon name="plus" color="#fff" size={18} />
            <Text style={s.newLabel}>New session</Text>
            <View style={{ flex: 1 }} />
            <Text style={s.model}>{working} working</Text>
          </Pressable>
        </View>
      )}
      <NativeSheet visible={sheet} onClose={() => setSheet(false)}>
        <KeyboardAvoidingView behavior="padding" style={{ flex: 1 }}>
          <RNScrollView
            keyboardShouldPersistTaps="handled"
            contentContainerStyle={{
              flexGrow: 1,
              padding: 20,
              paddingBottom: 36,
            }}
          >
            <View
              style={[
                s.row,
                { justifyContent: "space-between", paddingTop: 12 },
              ]}
            >
              <Pressable
                accessibilityLabel="Close composer"
                onPress={() => setSheet(false)}
                style={s.control}
              >
                <Icon name="xmark" />
              </Pressable>
              <Text
                style={{ color: "#f7f7fc", fontSize: 18, fontWeight: "600" }}
              >
                New Session
              </Text>
              <View style={{ width: 36 }} />
            </View>
            <View
              style={{
                height: 180,
                justifyContent: "center",
                alignItems: "center",
                gap: 16,
              }}
            >
              <Image source={logo} style={{ width: 56, height: 45 }} />
              <Text style={s.detailTitle}>What shall we build?</Text>
            </View>
            <RNScrollView
              horizontal
              showsHorizontalScrollIndicator={false}
              style={{ flexGrow: 0 }}
              contentContainerStyle={{ gap: 8, paddingBottom: 16 }}
            >
              {projects.map((p) => (
                <Pressable
                  key={p.id}
                  onPress={() => setProject(p.id)}
                  style={[s.chip, project === p.id && s.selected]}
                >
                  <Image source={p.avatar} style={{ width: 18, height: 18 }} />
                  <Text style={s.body}>{p.name}</Text>
                </Pressable>
              ))}
            </RNScrollView>
            <View
              style={{
                borderWidth: 1,
                borderColor: "#303036",
                backgroundColor: "#101113",
              }}
            >
              <View style={[s.checkout, { paddingHorizontal: 14 }]}>
                <Icon name="arrow.triangle.branch" size={16} />
                <Text style={s.model}>{project}/main</Text>
                <View style={{ flex: 1 }} />
                <Text style={s.time}>Current checkout</Text>
              </View>
              <View style={{ padding: 16 }}>
                <TextInput
                  multiline
                  value={prompt}
                  onChangeText={setPrompt}
                  placeholder="Ask Astrid, build…"
                  placeholderTextColor="#898996"
                  style={[s.prompt, { paddingTop: 0, minHeight: 72 }]}
                  accessibilityLabel="Session prompt"
                />
                <Text style={s.hint}>/ for commands, @ for references</Text>
                <View style={[s.row, { marginTop: 40, gap: 12 }]}>
                  <Pressable
                    style={[s.chip, { borderRadius: 0 }]}
                    onPress={() =>
                      setModel((m) =>
                        m === "GPT-6.1-Sol" ? "Opus 5.5" : "GPT-6.1-Sol",
                      )
                    }
                    accessibilityLabel="Switch model"
                  >
                    <Image source={logo} style={s.modelLogo} />
                    <Text style={s.body}>{model} · High</Text>
                    <Icon name="chevron.down" size={12} />
                  </Pressable>
                  <View style={{ flex: 1 }} />
                  <Pressable
                    accessibilityLabel="Attachment information"
                    onPress={() => {}}
                    disabled
                  >
                    <Icon name="paperclip" />
                  </Pressable>
                  <Pressable
                    accessibilityRole="button"
                    accessibilityLabel="Create demo session"
                    disabled={!prompt.trim()}
                    onPress={submit}
                    style={[
                      s.send,
                      { borderRadius: 0 },
                      !prompt.trim() && { opacity: 0.35 },
                    ]}
                  >
                    <Icon name="arrow.up" color="white" size={23} />
                  </Pressable>
                </View>
              </View>
            </View>
            <Text style={[s.eyebrow, { textAlign: "center", marginTop: 16 }]}>
              Local concept · No runtime connected
            </Text>
          </RNScrollView>
        </KeyboardAvoidingView>
      </NativeSheet>
    </View>
  );
}
function NativePage({
  page,
  title,
  onFilter,
  onArchive,
  children,
}: {
  page: string;
  title: string;
  onFilter: () => void;
  onArchive: () => void;
  children: React.ReactElement;
}) {
  const [collapsed, setCollapsed] = useState(false);
  const baseline = useRef<number | null>(null);
  const geometry = useScrollGeometryChange((g) => {
    if (baseline.current === null) baseline.current = g.contentOffsetY;
    setCollapsed(g.contentOffsetY - baseline.current > 40);
  });
  return (
    <NavigationStack>
      <Toolbar>
        <ZStack>
          <RNHostView>
            <ImageBackground
              source={require("../assets/background.png")}
              style={{ flex: 1 }}
              resizeMode="cover"
            >
              <View style={s.shade} />
            </ImageBackground>
          </RNHostView>
          <ScrollView
            showsIndicators={false}
            modifiers={[
              navigationTitle(title),
              navigationBarTitleDisplayMode("large"),
              scrollEdgeEffectStyle("soft", "top"),
              geometry!,
            ]}
          >
            <VStack spacing={0}>
              <RNHostView matchContents>
                <View style={{ paddingBottom: 180, minHeight: 850 }}>
                  {children}
                </View>
              </RNHostView>
            </VStack>
          </ScrollView>
        </ZStack>
        <Toolbar.Content>
          {page === "Home" && (
            <>
              <ToolbarItem
                placement={collapsed ? "topBarLeading" : "principal"}
              >
                <RNHostView matchContents>
                  <Image
                    source={logo}
                    style={{
                      width: collapsed ? 28 : 40,
                      height: collapsed ? 23 : 32,
                    }}
                    resizeMode="contain"
                  />
                </RNHostView>
              </ToolbarItem>
              <ToolbarItem placement="topBarTrailing">
                <Button
                  label="Archived sessions"
                  systemImage="tray"
                  onPress={onArchive}
                />
              </ToolbarItem>
              <ToolbarItem placement="topBarTrailing">
                <Button
                  label="Working sessions"
                  systemImage="line.3.horizontal.decrease"
                  onPress={onFilter}
                />
              </ToolbarItem>
            </>
          )}
        </Toolbar.Content>
      </Toolbar>
    </NavigationStack>
  );
}
