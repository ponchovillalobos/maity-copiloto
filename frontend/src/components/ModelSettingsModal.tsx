/**
 * Re-export del tipo `ModelConfig` por compatibilidad con código que lo importa
 * desde esta ruta. La fuente de verdad es `@/services/configService`.
 *
 * El componente UI fue eliminado: la app no expone selector de modelos. Los
 * modelos vienen pre-instalados con la app y se gestionan automáticamente.
 */
export type { ModelConfig } from '@/services/configService';
