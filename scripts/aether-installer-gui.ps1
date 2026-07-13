# aether-installer-gui.ps1 - Professional AETHER Setup Manager

Add-Type -AssemblyName PresentationFramework, PresentationCore, WindowsBase, System.Drawing

[xml]$xaml = @"
<Window xmlns="http://schemas.microsoft.com/winfx/2000/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2005/xaml"
        Title="AETHER Setup Manager" Height="420" Width="620" Background="#060913"
        WindowStartupLocation="CenterScreen" ResizeMode="NoResize">
    <Grid Margin="20">
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto"/>
            <RowDefinition Height="*"/>
            <RowDefinition Height="Auto"/>
        </Grid.RowDefinitions>

        <!-- Header Section -->
        <StackPanel Grid.Row="0" Margin="0,0,0,20">
            <TextBlock Text="AETHER Setup Manager" FontSize="26" FontWeight="ExtraBold" Foreground="#22d3ee" Margin="0,0,0,4"/>
            <TextBlock Text="Version 0.2.0 - Research-Oriented Programming Language" FontSize="13" Foreground="#94a3b8"/>
            <Separator Background="#22d3ee" Height="2" Margin="0,10,0,0" Opacity="0.3"/>
        </StackPanel>

        <!-- Main Body -->
        <Grid Grid.Row="1">
            <StackPanel x:Name="SetupPanel" Visibility="Visible" VerticalAlignment="Center">
                <TextBlock Text="This manager will compile AETHER, install the toolchain, and register file associations for .aether source files with the custom brand icon." 
                           Foreground="#e2e8f0" TextWrapping="Wrap" FontSize="14" Margin="0,0,0,20"/>
                
                <Border BorderBrush="#1e293b" BorderThickness="1" Background="#0f172a" CornerRadius="6" Padding="15" Margin="0,0,0,20">
                    <StackPanel>
                        <CheckBox x:Name="PathCheckbox" IsChecked="True" VerticalContentAlignment="Center">
                            <TextBlock Text="Add AETHER toolchain to environment variables (PATH)" Foreground="#e2e8f0" FontWeight="Bold"/>
                        </CheckBox>
                        <TextBlock Text="Recommended. Allows running 'aether' directly from command lines." 
                                   Foreground="#94a3b8" FontSize="11" Margin="20,4,0,0"/>
                    </StackPanel>
                </Border>

                <ProgressBar x:Name="ProgressBar" Height="15" Minimum="0" Maximum="100" Value="0" Background="#0f172a" Foreground="#8b5cf6" Visibility="Collapsed" Margin="0,0,0,10"/>
                <TextBlock x:Name="StatusLabel" Text="" Foreground="#22d3ee" FontSize="13" HorizontalAlignment="Center"/>
            </StackPanel>

            <!-- Success Panel -->
            <StackPanel x:Name="SuccessPanel" Visibility="Collapsed" VerticalAlignment="Center" HorizontalAlignment="Center">
                <TextBlock Text="[SUCCESS] Installation Complete!" FontSize="20" FontWeight="Bold" Foreground="#10b981" HorizontalAlignment="Center" Margin="0,0,0,10"/>
                <TextBlock Text="AETHER has been successfully compiled and installed on your laptop." Foreground="#e2e8f0" HorizontalAlignment="Center" Margin="0,0,0,4"/>
                <TextBlock Text="Registry file associations for .aether programs have been configured." Foreground="#e2e8f0" HorizontalAlignment="Center" Margin="0,0,0,20"/>
                <TextBlock Text="To verify, restart your terminal and type: aether --version" Foreground="#fbbf24" HorizontalAlignment="Center" FontWeight="Bold"/>
            </StackPanel>
        </Grid>

        <!-- Footer Actions -->
        <Grid Grid.Row="2">
            <Separator Background="#22d3ee" Height="1" VerticalAlignment="Top" Opacity="0.1"/>
            <StackPanel Orientation="Horizontal" HorizontalAlignment="Right" Margin="0,15,0,0">
                <Button x:Name="CancelButton" Content="Cancel" Width="100" Height="32" Margin="0,0,10,0" Background="#0f172a" Foreground="#e2e8f0" BorderBrush="#1e293b"/>
                <Button x:Name="InstallButton" Content="Install" Width="100" Height="32" Background="#22d3ee" Foreground="#060913" FontWeight="Bold" BorderThickness="0"/>
                <Button x:Name="FinishButton" Content="Finish" Width="100" Height="32" Background="#10b981" Foreground="#060913" FontWeight="Bold" BorderThickness="0" Visibility="Collapsed"/>
            </StackPanel>
        </Grid>
    </Grid>
</Window>
"@

# Read XAML layout
$reader = New-Object System.Xml.XmlNodeReader $xaml
$window = [Windows.Markup.XamlReader]::Load($reader)

# Get element controls
$setupPanel = $window.FindName("SetupPanel")
$successPanel = $window.FindName("SuccessPanel")
$pathCheckbox = $window.FindName("PathCheckbox")
$progressBar = $window.FindName("ProgressBar")
$statusLabel = $window.FindName("StatusLabel")
$cancelButton = $window.FindName("CancelButton")
$installButton = $window.FindName("InstallButton")
$finishButton = $window.FindName("FinishButton")

# Cancel Event
$cancelButton.Add_Click({
    $window.Close()
})

# Finish Event
$finishButton.Add_Click({
    $window.Close()
})

# Install Process Event
$installButton.Add_Click({
    $installButton.IsEnabled = $false
    $pathCheckbox.IsEnabled = $false
    $progressBar.Visibility = [System.Windows.Visibility]::Visible
    
    # 1. Update Status
    $statusLabel.Text = "Step 1/4: Checking compiler prerequisites..."
    $progressBar.Value = 25
    [System.Windows.Threading.DispatcherHeader]::Current | Out-Null
    Start-Sleep -Seconds 1

    # Check Cargo
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        [System.Windows.MessageBox]::Show("Rust or Cargo compiler was not found. Please install Rust from https://rustup.rs/ before running the installer.", "Prerequisite Missing", [System.Windows.MessageBoxButton]::OK, [System.Windows.MessageBoxImage]::Error)
        $window.Close()
        return
    }

    # 2. Build Release
    $statusLabel.Text = "Step 2/4: Compiling AETHER toolchain in release mode (cargo build)..."
    $progressBar.Value = 50
    Start-Sleep -Seconds 1
    
    # Compile
    cargo build --release
    
    # 3. Copy files & associations
    $statusLabel.Text = "Step 3/4: Staging binaries and registering file icons..."
    $progressBar.Value = 75
    Start-Sleep -Seconds 1

    $InstallDir = "$env:LOCALAPPDATA\aether"
    $BinDir = "$InstallDir\bin"
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

    # Copy binary
    Copy-Item "target\release\aether.exe" "$BinDir\aether.exe" -Force
    # Copy icon
    Copy-Item "logo\aether-logo.ico" "$InstallDir\aether-logo.ico" -Force

    # Registry associations
    New-Item -Path "HKCU:\Software\Classes\.aether" -Force | Out-Null
    Set-ItemProperty -Path "HKCU:\Software\Classes\.aether" -Name "(Default)" -Value "Aether.File" -Force
    New-Item -Path "HKCU:\Software\Classes\Aether.File" -Force | Out-Null
    Set-ItemProperty -Path "HKCU:\Software\Classes\Aether.File" -Name "(Default)" -Value "AETHER Program File" -Force
    New-Item -Path "HKCU:\Software\Classes\Aether.File\DefaultIcon" -Force | Out-Null
    Set-ItemProperty -Path "HKCU:\Software\Classes\Aether.File\DefaultIcon" -Name "(Default)" -Value "$InstallDir\aether-logo.ico" -Force

    # 4. PATH updates
    $statusLabel.Text = "Step 4/4: Configuring environment variables..."
    $progressBar.Value = 90
    Start-Sleep -Seconds 1

    if ($pathCheckbox.IsChecked) {
        $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($CurrentPath -notlike "*$BinDir*") {
            [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$BinDir", "User")
        }
    }
    [Environment]::SetEnvironmentVariable("AETHER_INSTALL_DIR", $InstallDir, "User")

    # Refresh Explorer
    $code = '[DllImport("shell32.dll")] public static extern void SHChangeNotify(int wEventId, int uFlags, IntPtr dwItem1, IntPtr dwItem2);'
    $type = Add-Type -MemberDefinition $code -Name "Shell32" -Namespace "WinAPI" -PassThru
    $type::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)

    # Success transition
    $setupPanel.Visibility = [System.Windows.Visibility]::Collapsed
    $successPanel.Visibility = [System.Windows.Visibility]::Visible
    $cancelButton.Visibility = [System.Windows.Visibility]::Collapsed
    $installButton.Visibility = [System.Windows.Visibility]::Collapsed
    $finishButton.Visibility = [System.Windows.Visibility]::Visible
})

# Launch GUI Setup Window
$window.ShowDialog() | Out-Null
