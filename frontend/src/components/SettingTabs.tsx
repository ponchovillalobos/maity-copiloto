import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { TranscriptModelProps, TranscriptSettings } from "./TranscriptSettings"
import { RecordingSettings } from "./RecordingSettings"
import { About } from "./About"
import { MeetingDetectorSettings } from "./MeetingDetectorSettings"

interface SettingTabsProps {
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    setSaveSuccess: (success: boolean | null) => void;
    defaultTab?: string;
}

export function SettingTabs({
    setSaveSuccess,
    defaultTab = "transcriptSettings",
    transcriptModelConfig,
    setTranscriptModelConfig,
}: SettingTabsProps) {

    const handleTabChange = () => {
        setSaveSuccess(null);
    };

    return (
        <Tabs defaultValue={defaultTab} className="w-full max-h-[calc(100vh-10rem)] overflow-y-auto" onValueChange={handleTabChange}>
  <TabsList>
    <TabsTrigger value="transcriptSettings">Transcripción</TabsTrigger>
    <TabsTrigger value="recordingSettings">Preferencias</TabsTrigger>
    <TabsTrigger value="meetingDetection">Reuniones</TabsTrigger>
    <TabsTrigger value="about">Acerca de</TabsTrigger>
  </TabsList>
<TabsContent value="transcriptSettings">
    <TranscriptSettings
    transcriptModelConfig={transcriptModelConfig}
    setTranscriptModelConfig={setTranscriptModelConfig}
  />
  </TabsContent>
  <TabsContent value="recordingSettings">
    <RecordingSettings />
  </TabsContent>
  <TabsContent value="meetingDetection">
    <MeetingDetectorSettings />
  </TabsContent>
  <TabsContent value="about">
    <About />
  </TabsContent>
</Tabs>
    )
}


