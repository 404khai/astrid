import type { ReactElement } from "react";
import { BottomSheet, Group, Host, RNHostView } from "@expo/ui/swift-ui";
import {
  presentationBackground,
  presentationDetents,
  presentationDragIndicator,
} from "@expo/ui/swift-ui/modifiers";
// Matches MonoCode mobile's NativeSheet.ios presentation contract.
export function NativeSheet({
  visible,
  onClose,
  children,
}: {
  visible: boolean;
  onClose: () => void;
  children: ReactElement;
}) {
  return (
    <Host
      style={{ position: "absolute", width: 0, height: 0 }}
      pointerEvents="box-none"
      colorScheme="dark"
    >
      <BottomSheet
        isPresented={visible}
        onIsPresentedChange={(open) => {
          if (!open) onClose();
        }}
      >
        <Group
          modifiers={[
            presentationDetents(["large"]),
            presentationDragIndicator("visible"),
            presentationBackground("#171717"),
          ]}
        >
          <RNHostView>{children}</RNHostView>
        </Group>
      </BottomSheet>
    </Host>
  );
}
