package version

import (
	"fmt"
	"runtime"
)

var (
	// Version is set at build time
	Version = "0.0.1a"
	// BuildDate is set at build time
	BuildDate = "unknown"
	// GitCommit is set at build time
	GitCommit = "unknown"
	// GitTag is set at build time
	GitTag = "unknown"
)

// BuildInfo represents build information
type BuildInfo struct {
	Version   string
	BuildDate string
	GitCommit string
	GitTag    string
	GoVersion string
}

// GetBuildInfo returns the current build information
func GetBuildInfo() BuildInfo {
	return BuildInfo{
		Version:   Version,
		BuildDate: BuildDate,
		GitCommit: GitCommit,
		GitTag:    GitTag,
		GoVersion: runtime.Version(),
	}
}

// GetVersion returns the current version
func GetVersion() string {
	return Version
}

// GetFullVersion returns a full version string with build info
func GetFullVersion() string {
	info := GetBuildInfo()
	if info.GitCommit != "unknown" && len(info.GitCommit) > 7 {
		info.GitCommit = info.GitCommit[:7]
	}

	if info.GitTag != "unknown" && info.GitTag != "" {
		return fmt.Sprintf("%s (%s)", info.GitTag, info.GitCommit)
	}

	return fmt.Sprintf("%s (%s)", info.Version, info.GitCommit)
}

// PrintVersionInfo prints detailed version information
func PrintVersionInfo(appName string) {
	info := GetBuildInfo()
	fmt.Printf("%s version %s\n", appName, info.Version)
	fmt.Printf("Build date: %s\n", info.BuildDate)
	fmt.Printf("Git commit: %s\n", info.GitCommit)
	fmt.Printf("Git tag: %s\n", info.GitTag)
	fmt.Printf("Go version: %s\n", info.GoVersion)
}
