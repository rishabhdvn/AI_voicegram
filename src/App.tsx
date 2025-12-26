import { useEffect, useState, useRef } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { createClient, LiveTranscriptionEvents } from "@deepgram/sdk";
import "./App.css"; // Make sure to import the CSS!

const DG_KEY = import.meta.env.VITE_DEEPGRAM_API_KEY;
const GM_KEY = import.meta.env.VITE_GEMINI_API_KEY;

function App() {
  const [status, setStatus] = useState<"idle" | "recording" | "processing">("idle");
  const transcriptBuffer = useRef("");
  const activeSession = useRef<any>(null);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    async function setup() {
      unlisten = await listen<string>("ptt-trigger", (e) => {
        if (e.payload === "start") handleStart();
        else handleStop();
      });
    }
    setup();
    return () => { if (unlisten) unlisten(); };
  }, []);

  const handleStart = async () => {
    if (!DG_KEY) return console.error("Missing Deepgram Key");
    setStatus("recording");
    transcriptBuffer.current = "";

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const dg = createClient(DG_KEY);
      const conn = dg.listen.live({ 
        model: "nova-2", 
        smart_format: true, 
        interim_results: false 
      });

      conn.on(LiveTranscriptionEvents.Open, () => {
        const recorder = new MediaRecorder(stream);
        recorder.ondataavailable = (e) => {
          if (e.data.size > 0 && conn.getReadyState() === 1) conn.send(e.data);
        };
        recorder.start(250);
        activeSession.current = { conn, stream, recorder };
      });

      conn.on(LiveTranscriptionEvents.Transcript, (data) => {
        const text = data.channel.alternatives[0].transcript;
        if (data.is_final && text) {
          transcriptBuffer.current += text + " ";
        }
      });
      
    } catch (err) {
      console.error(err);
      setStatus("idle");
    }
  };

  const handleStop = async () => {
    if (!activeSession.current) return;
    setStatus("processing"); // Show green processing state

    const { recorder, stream, conn } = activeSession.current;
    recorder.stop();
    stream.getTracks().forEach((t: any) => t.stop());
    conn.finish();
    activeSession.current = null;

    // Small delay to let final transcripts arrive
    setTimeout(async () => {
      try {
        await invoke("finalize_and_type", { 
          rawText: transcriptBuffer.current, 
          geminiKey: GM_KEY || "" 
        });
        setStatus("idle");
      } catch (err) {
        console.error("Rust Invoke Error:", err);
        setStatus("idle");
      }
    }, 500);
  };

  return (
    <div className={`container state-${status}`}>
      <div className="indicator"></div>
      <div className="status-text">{status.toUpperCase()}</div>
    </div>
  );
}

export default App;