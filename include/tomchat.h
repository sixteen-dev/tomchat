#ifndef TOMCHAT_H
#define TOMCHAT_H

#ifdef __cplusplus
extern "C" {
#endif

/// C-compatible error codes
typedef enum {
    TOMCHAT_SUCCESS = 0,
    TOMCHAT_ERROR = 1,
    TOMCHAT_INVALID_CONFIG = 2,
    TOMCHAT_AUDIO_ERROR = 3,
    TOMCHAT_TRANSCRIPTION_ERROR = 4
} TomChatResult;

/// Opaque handle for TomChat instance
typedef struct TomChatHandle TomChatHandle;

/**
 * Initialize TomChat with configuration file
 * @param config_path Path to configuration file (config.toml)
 * @return TomChat handle or NULL on error
 */
TomChatHandle* tomchat_init(const char* config_path);

/**
 * Start TomChat background service
 * @param handle TomChat handle
 * @return Result code
 */
int tomchat_start(TomChatHandle* handle);

/**
 * Stop TomChat service
 * @param handle TomChat handle
 * @return Result code
 */
int tomchat_stop(TomChatHandle* handle);

/**
 * Check if TomChat is currently running
 * @param handle TomChat handle
 * @return 1 if running, 0 if stopped
 */
int tomchat_is_running(const TomChatHandle* handle);

/**
 * Get last error message
 * @return Error message string (caller should not free)
 */
const char* tomchat_get_last_error(void);

/**
 * Set configuration parameter at runtime
 * @param handle TomChat handle
 * @param key Configuration key
 * @param value Configuration value
 * @return Result code
 */
int tomchat_set_config(TomChatHandle* handle, const char* key, const char* value);

/**
 * Cleanup and destroy TomChat handle
 * @param handle TomChat handle to destroy
 */
void tomchat_destroy(TomChatHandle* handle);

#ifdef __cplusplus
}
#endif

#endif // TOMCHAT_H