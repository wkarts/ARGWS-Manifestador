import { isNativeMobileRuntime, isTauriRuntime } from "../runtime/RuntimeProvider";
import { isMobileRemoteMode } from "../mobile/MobileConnection";
import type { CommandArgs, CommandProvider } from "./CommandProvider";
import { TauriCommandProvider } from "./TauriCommandProvider";
import { WebCommandProvider } from "./WebCommandProvider";
import { InternalApiCommandProvider, isInternalApiWebRuntime } from "./InternalApiCommandProvider";
import { MobileRemoteCommandProvider } from "./MobileRemoteCommandProvider";

let provider: CommandProvider | null = null;

export function resetCommandProvider(): void {
  provider = null;
}

export function useCommandProvider(): CommandProvider {
  if (!provider) {
    if (isNativeMobileRuntime() && isMobileRemoteMode()) {
      provider = new MobileRemoteCommandProvider();
    } else if (isTauriRuntime()) {
      provider = new TauriCommandProvider();
    } else if (isInternalApiWebRuntime()) {
      provider = new InternalApiCommandProvider();
    } else {
      provider = new WebCommandProvider();
    }
  }
  return provider;
}

export async function invokeAppCommand<T>(command: string, args?: CommandArgs): Promise<T> {
  return useCommandProvider().invoke<T>(command, args);
}
