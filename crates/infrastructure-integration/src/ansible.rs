//! Ansible playbook generation.

use std::fs;
use std::path::Path;
use serde_yaml;
use crate::config::{DeploymentConfig, AgentSpec};
use crate::error::InfrastructureError;

/// Generates Ansible playbooks and inventory files.
#[derive(Default)]
pub struct AnsibleGenerator;

impl AnsibleGenerator {
    /// Generate Ansible files in the specified output directory.
    pub async fn generate(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        let dir = Path::new(output_dir);
        fs::create_dir_all(dir)?;

        // Generate inventory
        let inventory = self.generate_inventory(config);
        fs::write(dir.join("inventory.yaml"), inventory)?;

        // Generate playbook
        let playbook = self.generate_playbook(config);
        fs::write(dir.join("deploy_agents.yaml"), playbook)?;

        // Generate vars file
        let vars = self.generate_vars(config);
        fs::write(dir.join("group_vars/all.yaml"), vars)?;

        // Generate requirements.txt for ansible-galaxy (optional)
        let requirements = self.generate_requirements();
        fs::write(dir.join("requirements.yaml"), requirements)?;

        tracing::info!("Ansible configuration generated in {}", output_dir);
        Ok(())
    }

    fn generate_inventory(&self, config: &DeploymentConfig) -> String {
        let mut inventory = String::from("---\n");
        inventory.push_str("all:\n  hosts:\n");
        // For simplicity, we generate placeholder hostnames.
        // In a real scenario, you would use Terraform outputs or static IPs.
        for (idx, spec) in config.agents.iter().enumerate() {
            for i in 0..spec.count {
                inventory.push_str(&format!("    agent_{}_{}:\n", idx, i));
                inventory.push_str(&format!("      ansible_host: 10.0.{}.{}\n", idx, i + 1));
                inventory.push_str("      ansible_user: ubuntu\n");
                inventory.push_str("      ansible_ssh_private_key_file: ~/.ssh/id_rsa\n");
            }
        }
        inventory.push_str("  vars:\n");
        inventory.push_str(&format!("    deployment_region: {}\n", config.region));
        inventory.push_str(&format!("    provider: {:?}\n", config.provider));
        inventory
    }

    fn generate_playbook(&self, config: &DeploymentConfig) -> String {
        let mut playbook = String::from("---\n");
        playbook.push_str("- name: Deploy offline‑first multi‑agent system\n");
        playbook.push_str("  hosts: all\n");
        playbook.push_str("  become: yes\n");
        playbook.push_str("  tasks:\n");

        // Common tasks
        playbook.push_str(
            r#"  - name: Update apt cache
    apt:
      update_cache: yes
      cache_valid_time: 3600
    when: ansible_os_family == 'Debian'

  - name: Install Docker dependencies
    apt:
      name:
        - apt-transport-https
        - ca-certificates
        - curl
        - software-properties-common
      state: present
    when: ansible_os_family == 'Debian'

  - name: Add Docker GPG key
    apt_key:
      url: https://download.docker.com/linux/ubuntu/gpg
      state: present
    when: ansible_os_family == 'Debian'

  - name: Add Docker repository
    apt_repository:
      repo: deb [arch=amd64] https://download.docker.com/linux/ubuntu {{ ansible_distribution_release }} stable
      state: present
    when: ansible_os_family == 'Debian'

  - name: Install Docker
    apt:
      name: docker-ce
      state: present
    when: ansible_os_family == 'Debian'

  - name: Start and enable Docker
    systemd:
      name: docker
      state: started
      enabled: yes

  - name: Create agent directory
    file:
      path: /opt/offline-first-agent
      state: directory
      mode: '0755'

  - name: Copy agent configuration
    template:
      src: agent_config.json.j2
      dest: /opt/offline-first-agent/config.json
      mode: '0644'

  - name: Pull agent Docker image
    docker_image:
      name: "{{ agent_image }}"
      tag: latest
      source: pull

  - name: Run agent container
    docker_container:
      name: offline-first-agent
      image: "{{ agent_image }}:latest"
      state: started
      restart_policy: always
      env: "{{ agent_env }}"
      command: "{{ agent_args }}"
      network_mode: host
      volumes:
        - /opt/offline-first-agent/config.json:/config.json
"#,
        );

        // Additional provider‑specific tasks
        match config.provider {
            crate::config::CloudProvider::Kubernetes => {
                playbook.push_str(
                    r#"
  - name: Install kubectl
    apt:
      name: kubectl
      state: present
    when: ansible_os_family == 'Debian'
"#,
                );
            }
            _ => {}
        }

        playbook
    }

    fn generate_vars(&self, config: &DeploymentConfig) -> String {
        let mut vars = serde_yaml::Mapping::new();
        vars.insert(
            serde_yaml::Value::String("agent_image".to_string()),
            serde_yaml::Value::String(
                config
                    .agents
                    .first()
                    .map(|spec| spec.image.clone())
                    .unwrap_or_else(|| "offline‑first‑agent:latest".to_string()),
            ),
        );
        vars.insert(
            serde_yaml::Value::String("agent_env".to_string()),
            serde_yaml::Value::Sequence(
                config
                    .agents
                    .first()
                    .map(|spec| {
                        spec.env
                            .iter()
                            .map(|(k, v)| {
                                serde_yaml::Value::String(format!("{}={}", k, v))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
        );
        vars.insert(
            serde_yaml::Value::String("agent_args".to_string()),
            serde_yaml::Value::String(
                config
                    .agents
                    .first()
                    .map(|spec| spec.args.join(" "))
                    .unwrap_or_default(),
            ),
        );
        vars.insert(
            serde_yaml::Value::String("deployment_region".to_string()),
            serde_yaml::Value::String(config.region.clone()),
        );
        serde_yaml::to_string(&vars).unwrap()
    }

    fn generate_requirements(&self) -> String {
        r#"---
collections:
  - name: community.docker
    version: ">=3.0.0"
  - name: kubernetes.core
    version: ">=2.0.0"
"#.to_string()
    }
}