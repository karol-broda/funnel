package version

import (
	"fmt"
	"runtime"
)

var (
	Version   = "0.0.5"
	BuildDate = "unknown"
	GitCommit = "unknown"
	GitTag    = "unknown"
)

type BuildInfo struct {
	Version   string
	BuildDate string
	GitCommit string
	GitTag    string
	GoVersion string
}

func GetBuildInfo() BuildInfo {
	return BuildInfo{
		Version:   Version,
		BuildDate: BuildDate,
		GitCommit: GitCommit,
		GitTag:    GitTag,
		GoVersion: runtime.Version(),
	}
}

func GetVersion() string {
	return Version
}

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

func PrintVersionInfo(appName string) {
	info := GetBuildInfo()
	fmt.Printf("%s version %s\n", appName, info.Version)
	fmt.Printf("Build date: %s\n", info.BuildDate)
	fmt.Printf("Git commit: %s\n", info.GitCommit)
	fmt.Printf("Git tag: %s\n", info.GitTag)
	fmt.Printf("Go version: %s\n", info.GoVersion)
}
