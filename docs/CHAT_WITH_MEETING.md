# Chat Conversacional sobre Reuniones

## Descripción General

Interfaz conversacional que permite hacer preguntas en lenguaje natural sobre lo dicho en una reunión grabada. La IA responde citando literalmente con timestamps `[MM:SS]` donde se mencionó cada detalle. Resultados basados en búsqueda semántica local.

**Características:**
- Búsqueda por similitud semántica (embedding local)
- Citas literales con timestamps precisos
- Índice automático tras finalizar grabación
- 100% privado — Ollama local
- Multi-turno conversacional con historial
- Preguntas sugeridas precargadas

## Arquitectura

### Backend Rust

**Módulo principal:**
```
frontend/src-tauri/src/coach/meeting_chat.rs (~280 LOC)
```

**Comando Tauri:**
```rust
#[tauri::command]
async fn chat_with_meeting(
    message: String,
    meeting_id: String,
    history: Option<Vec<ChatTurn>>,
    model: Option<String>
) -> Result<ChatResponse, String>
```

**Parámetros:**
- `message`: Pregunta/mensaje del usuario (ej: "¿Qué acuerdos se alcanzaron?")
- `meeting_id`: ID de la reunión a consultar
- `history`: Turnos previos conversación (max 20 para contexto)
- `model`: Modelo LLM Ollama (fallback a `gemma3:4b`)

**Retorno:**
```rust
pub struct ChatResponse {
    pub answer: String,               // Respuesta completa
    pub citations: Vec<Citation>,     // [MM:SS] timestamps dentro del answer
    pub model: String,                // Modelo usado
    pub latency_ms: u64,              // Tiempo de respuesta
    pub context_chars: usize,         // Chars de contexto usado
    pub context_turns: usize,         // Num turnos incluidos
    pub user_turns: usize,            // Turnos del usuario en contexto
    pub interlocutor_turns: usize,    // Turnos del interlocutor
}
```

### Búsqueda Semántica

**Reuso de módulo existente:**
```
frontend/src-tauri/src/semantic_search/repository.rs
```

**Proceso:**
1. Embeddings de la pregunta con `nomic-embed-text` (~768 dims, 0.2s)
2. Cargaar top-5 segmentos de `transcript_embeddings` tabla filtrando `meeting_id`
3. Calcular cosine similarity entre pregunta y cada segmento
4. Ordenar por score descendente
5. Pasar fragmentos al LLM como contexto

**Tabla de persistencia:**
```sql
CREATE TABLE transcript_embeddings (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    segment_text TEXT NOT NULL,           -- Fragmento (ej: 500 chars)
    timestamp_start INTEGER,              -- Milisegundos en la grabación
    timestamp_end INTEGER,
    embedding BLOB NOT NULL,              -- Vector 768d en SQLite BLOB format
    speaker TEXT,                         -- "user" | "interlocutor"
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
```

### Frontend TypeScript

**Componente principal:**
```typescript
// frontend/src/components/MeetingChat/MeetingChatPanel.tsx (~320 LOC)
export function MeetingChatPanel({ meetingId }: Props) {
  const [messages, setMessages] = useState<ChatMessage[]>(preloadedQuestions);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  
  const handleSendMessage = async (text: string) => {
    const response = await invoke('chat_with_meeting', {
      message: text,
      meeting_id: meetingId,
      history: messages.filter(m => m.role !== 'system').slice(-10)
    });
    
    setMessages(prev => [
      ...prev,
      { role: 'user', content: text },
      { role: 'assistant', content: response.answer, citations: response.citations }
    ]);
  };
}
```

**Preguntas sugeridas (hardcoded):**
```typescript
const SUGGESTED_QUESTIONS = [
  "¿Qué acuerdos se alcanzaron?",
  "¿Qué objeciones surgieron?",
  "¿Cuáles fueron las próximas acciones?",
  "¿Qué temas quedaron pendientes?",
  "¿Cómo fue el tono de la conversación?",
  "¿Cuáles fueron los puntos principales de desacuerdo?"
];
```

Preguntas se muestran como chips clickables al inicio, desaparecen tras primer mensaje.

## Flujo de Uso

### Índice Automático

1. **Usuario presiona "Detener grabación"** → `stop_recording` invocado
2. **Backend llama** `semantic_index_meeting(meeting_id)` fire-and-forget
3. **Indexación en background** (~3-5 seg para reunión de 15 min):
   - Dividir transcripción en segmentos 500 chars con overlap 50 chars
   - Generar embedding cada segmento vía Ollama `nomic-embed-text`
   - Guardar en tabla `transcript_embeddings`
4. **Usuario navega a detalles meeting** sin esperar → tab "Chat" disponible
5. **Si indexación aún no terminó**, mostrar mensaje "Indexando..."

### Chat Conversacional

1. **Usuario abre tab "Chat"** en detalles meeting
2. **Ve preguntas sugeridas** (6 chips)
3. **Hace clic pregunta O escribe propia** → `chat_with_meeting` invocado
4. **Backend:**
   - Genera embedding de la pregunta
   - Busca top-5 segmentos similares en BD
   - Construye prompt con fragmentos como contexto
   - Invoca LLM (Ollama `gemma3:4b` default)
   - Parsea respuesta e identifica timestamps `[MM:SS]`
5. **Frontend renderiza respuesta** con:
   - Texto completo de la respuesta
   - Chips clickables para cada `[MM:SS]` → salta a ese timestamp en transcripción
6. **Usuario puede hacer seguimiento** ("Explica más sobre...", "¿Por qué?")
7. **Historial se mantiene** (scroll) — máx 20 turnos para no saturar contexto

## Estructura del Prompt

**Template:**
```
System:
Eres un asistente experto en reuniones que responde preguntas basadas en la transcripción proporcionada.
IMPORTANTE: Cita SIEMPRE los momentos específicos donde se menciona lo que describes usando [MM:SS].
Sé conciso pero completo. Si la respuesta no está en la transcripción, di claramente "No se menciona en la reunión".

Context (top-5 segmentos relevantes):
[Segmento 1] [03:42 - User] "...texto...",
[Segmento 2] [05:18 - Interlocutor] "...texto...",
...

Pregunta del usuario:
{user_question}

Respuesta (incluir [MM:SS] para cada cita):
```

**Salida esperada:**
```
Se mencionaron varios acuerdos [04:15]:
- Revisar propuesta [04:15]
- Contactar al equipo legal [06:32]
- Próxima reunión el jueves [08:05]
```

## Modelos LLM Requeridos

**Obligatorio:**
- `nomic-embed-text` (~270MB) — embedding local via Ollama
  ```bash
  ollama pull nomic-embed-text
  ```

**Recomendado:**
- `gemma3:4b` (~3GB) — respuestas rápidas y precisas
  ```bash
  ollama pull gemma3:4b
  ```

**Alternativa premium:**
- `gemma4:e4b` (~7GB) — respuestas más detalladas
  ```bash
  ollama pull gemma4:e4b
  ```

Si usuario no tiene embedding model, mostrar error claro con instrucciones descarga.

## API Tauri

### Comando Principal
```rust
#[tauri::command]
async fn chat_with_meeting(
    message: String,
    meeting_id: String,
    history: Option<Vec<ChatTurn>>,
    model: Option<String>
) -> Result<ChatResponse, String>
```

### Comando Auxiliar (Índice)
```rust
#[tauri::command]
async fn semantic_index_meeting(meeting_id: String) -> Result<(), String>
```
Índice manual si el automático no se completó. Bloquea hasta que termine.

```rust
#[tauri::command]
async fn clear_meeting_index(meeting_id: String) -> Result<(), String>
```
Borra embeddings de una reunión (ej: para re-indexar).

```rust
#[tauri::command]
async fn get_embedding_status(meeting_id: String) -> Result<EmbeddingStatus, String>
```
Retorna estado: `{ indexed: bool, segment_count: int, created_at: DateTime, last_query_at: Option<DateTime> }`

## Persistencia en BD

**Tabla principal (existente):**
```sql
CREATE TABLE transcript_embeddings (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    segment_text TEXT NOT NULL,
    timestamp_start INTEGER,
    timestamp_end INTEGER,
    embedding BLOB,
    speaker TEXT,
    created_at DATETIME,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
```

**Índices recomendados:**
```sql
CREATE INDEX idx_embedding_meeting ON transcript_embeddings(meeting_id);
CREATE INDEX idx_embedding_speaker ON transcript_embeddings(speaker);
```

**Cálculo de cosine similarity en SQL** (via Rust, no SQL puro):
```rust
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot_product / (norm_a * norm_b)
}
```

## Componente Frontend

**MeetingChatPanel.tsx estructura:**

```typescript
interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  citations?: Citation[];
  timestamp?: number;
}

interface Citation {
  timestamp: string;   // "MM:SS"
  ms: number;         // ms absoluto
  text: string;       // fragmento citado
}

function MeetingChatPanel({ meetingId }: { meetingId: string }) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [suggestedQuestions, setSuggestedQuestions] = useState(SUGGESTED_QUESTIONS);
  const [indexStatus, setIndexStatus] = useState<EmbeddingStatus | null>(null);
  const [loading, setLoading] = useState(false);
  
  // Effects:
  // - Cargar estado índice al montar
  // - Suscribirse a cambios
  // - Scroll automático al agregar mensaje
  
  // Handlers:
  // - handleClickSuggestedQuestion(q) → limpiar sugerencias + enviar
  // - handleSendMessage(text) → invoke chat_with_meeting
  // - handleCitationClick(ms) → saltar a timestamp en transcripción
  
  return (
    <div className="chat-panel">
      {!indexStatus?.indexed && (
        <div className="banner-indexing">
          Indexando transcripción... {indexStatus?.segment_count} segmentos
        </div>
      )}
      
      {suggestedQuestions.length > 0 && (
        <div className="suggested-questions">
          {suggestedQuestions.map(q => (
            <button key={q} onClick={() => handleClickSuggestedQuestion(q)}>
              {q}
            </button>
          ))}
        </div>
      )}
      
      <div className="messages-list">
        {messages.map(msg => (
          <div key={msg.id} className={`message ${msg.role}`}>
            {msg.content}
            {msg.citations?.map(cit => (
              <button
                key={cit.ms}
                onClick={() => handleCitationClick(cit.ms)}
                className="citation-chip"
              >
                {cit.timestamp}
              </button>
            ))}
          </div>
        ))}
      </div>
      
      <div className="input-area">
        <input
          type="text"
          placeholder="Haz una pregunta sobre la reunión..."
          disabled={!indexStatus?.indexed || loading}
        />
        <button onClick={() => handleSendMessage(input)}>
          Enviar
        </button>
      </div>
    </div>
  );
}
```

## Ejemplo Flujo Completo

**Escenario:**
1. Usuario graba reunión de 12 minutos: "¿Qué acordamos sobre budget?"
2. Presiona "Detener"
3. `semantic_index_meeting('meeting-123')` iniciado en background
4. Navega a detalles → tab Chat
5. Ve sugerencias (aún indexando)
6. Espera ~4 seg, aparece "Listo"
7. Hace clic "¿Qué acuerdos se alcanzaron?"
8. Backend busca top-5 segmentos sobre "acuerdos"
9. LLM genera respuesta:
   ```
   Se alcanzaron los siguientes acuerdos:
   
   1. Aumentar budget marketing 15% [02:34]
   2. Revisar propuesta de Javier para Q2 [04:18]
   3. Reunión de seguimiento el próximo viernes [09:12]
   ```
10. Usuario hace clic `[04:18]` → transcripción salta a ese timestamp
11. Usuario pregunta: "¿Por cuánto fue el aumento?"
12. Backend mantiene historial (mensaje anterior + contexto) → respuesta más precisa

## Testing

**Tests Rust** (`cargo test --lib coach::meeting_chat`):
- Chat basic (pregunta → respuesta)
- Multi-turn (conversación 3-5 turnos)
- Citation parsing (detecta `[MM:SS]` correctamente)
- Índice missing (error claro si no indexado)
- Historia truncada (máx 20 turnos)
- Modelo fallback (usa default si no especificado)

**Test Manual:**
1. Grabar reunión 10+ min
2. Detener grabación
3. Abrir detalles meeting → tab Chat
4. Esperar a que muestre "Indexado"
5. Hacer clic pregunta sugerida
6. Validar respuesta + timestamps clickables
7. Hacer seguimiento (multi-turn)

## Limitaciones Conocidas

- **Modelos requeridos**: Requiere tanto `nomic-embed-text` como `gemma3:4b` instalados en Ollama
- **Latencia inicial**: Primer mensaje ~10-15s (carga modelo si está en disco frio)
- **Tamaño contexto**: Máx 5 segmentos (limit hardcoded; aumentar reduce velocidad)
- **Precisión**: Embeddings más precisos en español que en idiomas minoritarios
- **Timestamps**: Alineación puede ser ~2-3 seg off si grabación tiene gaps

## Mejoras Futuras

- Análisis de sentimiento por segmento (colores en gráfico)
- Exportación de chat a PDF con citas
- Recomendaciones automáticas post-reunión basadas en chat
- Filtrar respuestas por speaker (preguntar "¿Qué dijo el usuario sobre...?")
- Buscar por entidad (personas, lugares, productos mencionados)
- Comparación multi-reunión ("¿Se mencionó lo mismo en la reunión anterior?")
