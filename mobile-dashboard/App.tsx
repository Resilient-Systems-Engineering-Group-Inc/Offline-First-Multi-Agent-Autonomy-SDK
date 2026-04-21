/**
 * Mobile Dashboard for Multi-Agent SDK
 * @flow
 */

import React, { useEffect, useState } from 'react';
import {
  SafeAreaView,
  ScrollView,
  View,
  Text,
  StyleSheet,
  ActivityIndicator,
  RefreshControl,
  FlatList,
  TouchableOpacity,
  StatusBar,
} from 'react-native';
import { NavigationContainer } from '@react-navigation/native';
import { createStackNavigator } from '@react-navigation/stack';
import Icon from 'react-native-vector-icons/Ionicons';

import { TaskListScreen } from './src/screens/TaskListScreen';
import { AgentListScreen } from './src/screens/AgentListScreen';
import { MetricsScreen } from './src/screens/MetricsScreen';
import { TaskDetailScreen } from './src/screens/TaskDetailScreen';

const Stack = createStackNavigator();

const API_URL = 'http://localhost:3000/api';

function App(): React.JSX.Element {
  return (
    <NavigationContainer>
      <Stack.Navigator
        initialRouteName="Home"
        screenOptions={{
          headerStyle: {
            backgroundColor: '#1a73e8',
          },
          headerTintColor: '#fff',
          headerTitleStyle: {
            fontWeight: 'bold',
          },
        }}
      >
        <Stack.Screen
          name="Home"
          component={HomeScreen}
          options={{
            title: 'SDK Dashboard',
            headerShown: false,
          }}
        />
        <Stack.Screen
          name="TaskDetail"
          component={TaskDetailScreen}
          options={({ route }) => ({
            title: 'Task Details',
          })}
        />
      </Stack.Navigator>
    </NavigationContainer>
  );
}

function HomeScreen({ navigation }) {
  const [metrics, setMetrics] = useState(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const fetchMetrics = async () => {
    try {
      const response = await fetch(`${API_URL}/metrics`);
      const data = await response.json();
      setMetrics(data);
    } catch (error) {
      console.error('Failed to fetch metrics:', error);
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    fetchMetrics();
  }, []);

  const onRefresh = () => {
    setRefreshing(true);
    fetchMetrics();
  };

  if (loading) {
    return (
      <SafeAreaView style={styles.centerContainer}>
        <ActivityIndicator size="large" color="#1a73e8" />
        <Text style={styles.loadingText}>Loading...</Text>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      <StatusBar barStyle="light-content" backgroundColor="#1a73e8" />
      <ScrollView
        contentContainerStyle={styles.scrollContent}
        refreshControl={
          <RefreshControl refreshing={refreshing} onRefresh={onRefresh} />
        }
      >
        <View style={styles.header}>
          <Text style={styles.headerTitle}>Multi-Agent SDK</Text>
          <Text style={styles.headerSubtitle}>Real-time Monitoring</Text>
        </View>

        {metrics && (
          <>
            <View style={styles.statsContainer}>
              <View style={styles.statCard}>
                <Icon name="robot" size={32} color="#4CAF50" />
                <Text style={styles.statValue}>{metrics.totalAgents}</Text>
                <Text style={styles.statLabel}>Total Agents</Text>
              </View>

              <View style={styles.statCard}>
                <Icon name="wifi" size={32} color="#2196F3" />
                <Text style={styles.statValue}>{metrics.activeAgents}</Text>
                <Text style={styles.statLabel}>Active Agents</Text>
              </View>

              <View style={styles.statCard}>
                <Icon name="list" size={32} color="#FF9800" />
                <Text style={styles.statValue}>{metrics.totalTasks}</Text>
                <Text style={styles.statLabel}>Total Tasks</Text>
              </View>

              <View style={styles.statCard}>
                <Icon name="checkmark-circle" size={32} color="#9C27B0" />
                <Text style={styles.statValue}>{metrics.completedTasks}</Text>
                <Text style={styles.statLabel}>Completed</Text>
              </View>
            </View>

            <View style={styles.menuContainer}>
              <TouchableOpacity
                style={styles.menuItem}
                onPress={() => navigation.navigate('TaskList')}
              >
                <Icon name="tasks" size={24} color="#1a73e8" />
                <Text style={styles.menuItemText}>Tasks</Text>
                <Icon name="chevron-forward" size={20} color="#999" />
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.menuItem}
                onPress={() => navigation.navigate('AgentList')}
              >
                <Icon name="apps" size={24} color="#1a73e8" />
                <Text style={styles.menuItemText}>Agents</Text>
                <Icon name="chevron-forward" size={20} color="#999" />
              </TouchableOpacity>

              <TouchableOpacity
                style={styles.menuItem}
                onPress={() => navigation.navigate('Metrics')}
              >
                <Icon name="stats-chart" size={24} color="#1a73e8" />
                <Text style={styles.menuItemText}>Metrics</Text>
                <Icon name="chevron-forward" size={20} color="#999" />
              </TouchableOpacity>
            </View>
          </>
        )}
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  centerContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  loadingText: {
    marginTop: 16,
    fontSize: 16,
    color: '#666',
  },
  scrollContent: {
    padding: 16,
  },
  header: {
    marginBottom: 24,
  },
  headerTitle: {
    fontSize: 28,
    fontWeight: 'bold',
    color: '#333',
  },
  headerSubtitle: {
    fontSize: 16,
    color: '#666',
    marginTop: 4,
  },
  statsContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    justifyContent: 'space-between',
    marginBottom: 24,
  },
  statCard: {
    width: '48%',
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
    alignItems: 'center',
    elevation: 2,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
  },
  statValue: {
    fontSize: 24,
    fontWeight: 'bold',
    marginTop: 8,
    color: '#333',
  },
  statLabel: {
    fontSize: 12,
    color: '#666',
    marginTop: 4,
  },
  menuContainer: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 8,
    elevation: 2,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
  },
  menuItem: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 16,
    borderBottomWidth: 1,
    borderBottomColor: '#eee',
  },
  menuItemText: {
    flex: 1,
    fontSize: 16,
    marginLeft: 12,
    color: '#333',
  },
});

export default App;
