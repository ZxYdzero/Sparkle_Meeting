package config

import (
	"fmt"
	"os"
	"path/filepath"
	"sync"

	"gopkg.in/yaml.v3"
)

// Config SFU 配置
type Config struct {
	// UDP 端口范围
	UDPPortMin int `yaml:"udp_port_min" env:"UDP_PORT_MIN"`
	UDPPortMax int `yaml:"udp_port_max" env:"UDP_PORT_MAX"`

	// 公网 IP
	PublicIP string `yaml:"public_ip" env:"PUBLIC_IP"`

	mu sync.RWMutex
}

// 默认配置
var defaultConfig = &Config{
	UDPPortMin: 50000,
	UDPPortMax: 60000,
	PublicIP:   "",
}

// 全局配置实例
var globalConfig *Config
var globalConfigOnce sync.Once

// Get 获取全局配置
func Get() *Config {
	globalConfigOnce.Do(func() {
		cfg := &Config{
			UDPPortMin: defaultConfig.UDPPortMin,
			UDPPortMax: defaultConfig.UDPPortMax,
			PublicIP:   defaultConfig.PublicIP,
		}

		// 尝试从配置文件加载
		paths := []string{
			"config.yaml",
			"config.yml",
			os.Getenv("SFU_CONFIG_PATH"),
		}

		for _, path := range paths {
			if path == "" {
				continue
			}
			if err := loadFromFile(cfg, path); err == nil {
				break
			}
		}

		// 环境变量覆盖
		if v := os.Getenv("UDP_PORT_MIN"); v != "" {
			fmt.Sscanf(v, "%d", &cfg.UDPPortMin)
		}
		if v := os.Getenv("UDP_PORT_MAX"); v != "" {
			fmt.Sscanf(v, "%d", &cfg.UDPPortMax)
		}
		if v := os.Getenv("PUBLIC_IP"); v != "" {
			cfg.PublicIP = v
		}

		globalConfig = cfg
	})

	return globalConfig
}

// loadFromFile 从文件加载配置
func loadFromFile(cfg *Config, path string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}

	ext := filepath.Ext(path)
	if ext == ".yaml" || ext == ".yml" {
		if err := yaml.Unmarshal(data, cfg); err != nil {
			return fmt.Errorf("解析 YAML 失败: %w", err)
		}
	}
	return nil
}

// GetUDPPortRange 获取 UDP 端口范围
func (c *Config) GetUDPPortRange() (int, int) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.UDPPortMin, c.UDPPortMax
}

// GetPublicIP 获取公网 IP
func (c *Config) GetPublicIP() string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.PublicIP
}

// Validate 验证配置
func (c *Config) Validate() error {
	if c.UDPPortMin <= 0 || c.UDPPortMax <= 0 {
		return fmt.Errorf("UDP 端口范围无效")
	}
	if c.UDPPortMin > c.UDPPortMax {
		return fmt.Errorf("UDP 端口最小值不能大于最大值")
	}
	return nil
}
