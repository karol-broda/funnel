package client

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/BurntSushi/toml"
)

type Config struct {
	Inlets map[string]Inlet `toml:"inlets"`
}

// Inlet represents a tunnel server configuration
type Inlet struct {
	Server string `toml:"server"`
	Domain string `toml:"domain,omitempty"`
}

// ConfigManager handles configuration loading and management
type ConfigManager struct {
	configPath string
	config     *Config
}

// NewConfigManager creates a new configuration manager
func NewConfigManager() *ConfigManager {
	return &ConfigManager{}
}

// getConfigPath determines the configuration file path
func (cm *ConfigManager) getConfigPath() string {
	if cm.configPath != "" {
		return cm.configPath
	}

	// try XDG config directory first
	if configDir := os.Getenv("XDG_CONFIG_HOME"); configDir != "" {
		cm.configPath = filepath.Join(configDir, "funnel", "config.toml")
		return cm.configPath
	}

	// fallback to ~/.config/funnel/config.toml
	if homeDir, err := os.UserHomeDir(); err == nil {
		configDir := filepath.Join(homeDir, ".config", "funnel")
		cm.configPath = filepath.Join(configDir, "config.toml")
		return cm.configPath
	}

	// if all else fails, use current directory
	cm.configPath = "funnel.toml"
	return cm.configPath
}

// LoadConfig loads the configuration from file
func (cm *ConfigManager) LoadConfig() (*Config, error) {
	configPath := cm.getConfigPath()

	// check if config file exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("config file not found at %s", configPath)
	}

	// read and parse existing config
	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file %s: %w", configPath, err)
	}

	config := &Config{
		Inlets: make(map[string]Inlet),
	}

	if err := toml.Unmarshal(data, config); err != nil {
		return nil, fmt.Errorf("failed to parse config file %s: %w", configPath, err)
	}

	cm.config = config
	return config, nil
}

// SaveConfig saves the configuration to file
func (cm *ConfigManager) SaveConfig(config *Config) error {
	configPath := cm.getConfigPath()

	// ensure the directory exists
	configDir := filepath.Dir(configPath)
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return fmt.Errorf("failed to create config directory %s: %w", configDir, err)
	}

	// marshal config to TOML
	data, err := toml.Marshal(config)
	if err != nil {
		return fmt.Errorf("failed to marshal config to TOML: %w", err)
	}

	// write to file
	if err := os.WriteFile(configPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write config file %s: %w", configPath, err)
	}

	cm.config = config
	return nil
}

// GetInlet retrieves a specific inlet configuration by name
func (cm *ConfigManager) GetInlet(name string) (*Inlet, error) {
	if cm.config == nil {
		if _, err := cm.LoadConfig(); err != nil {
			return nil, err
		}
	}

	inlet, exists := cm.config.Inlets[name]
	if !exists {
		return nil, fmt.Errorf("inlet '%s' not found in configuration", name)
	}

	if inlet.Server == "" {
		return nil, fmt.Errorf("inlet '%s' has no server configured", name)
	}

	return &inlet, nil
}

// GetDefaultInlet retrieves the default inlet configuration
func (cm *ConfigManager) GetDefaultInlet() (*Inlet, error) {
	return cm.GetInlet("default")
}

// ListInlets returns all available inlet names
func (cm *ConfigManager) ListInlets() ([]string, error) {
	if cm.config == nil {
		if _, err := cm.LoadConfig(); err != nil {
			return nil, err
		}
	}

	inlets := make([]string, 0, len(cm.config.Inlets))
	for name := range cm.config.Inlets {
		inlets = append(inlets, name)
	}

	return inlets, nil
}

// ValidateConfig validates the loaded configuration
func (cm *ConfigManager) ValidateConfig() error {
	if cm.config == nil {
		return fmt.Errorf("no configuration loaded")
	}

	if len(cm.config.Inlets) == 0 {
		return fmt.Errorf("no inlets configured")
	}

	for name, inlet := range cm.config.Inlets {
		if inlet.Server == "" {
			return fmt.Errorf("inlet '%s' has no server configured", name)
		}
	}

	return nil
}
