const { createRunOncePlugin, IOSConfig, AndroidConfig } = require("@expo/config-plugins");
const pkg = require("./package.json");

const MICROPHONE_USAGE = "Allow $(PRODUCT_NAME) to access the microphone.";

const withAudioOpus = (config, { microphonePermission } = {}) => {
  if (microphonePermission !== false) {
    config = AndroidConfig.Permissions.withPermissions(config, [
      "android.permission.RECORD_AUDIO",
    ]);
  }

  return IOSConfig.Permissions.createPermissionsPlugin({
    NSMicrophoneUsageDescription: MICROPHONE_USAGE,
  })(config, {
    NSMicrophoneUsageDescription: microphonePermission,
  });
};

module.exports = createRunOncePlugin(withAudioOpus, pkg.name, pkg.version);
